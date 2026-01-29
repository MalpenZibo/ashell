use cosmic_protocols::image_capture_source::v1::client::zcosmic_workspace_image_capture_source_manager_v1;
use std::time::Duration;
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};
use wayland_protocols::ext::{
    image_capture_source::v1::client::{
        ext_foreign_toplevel_image_capture_source_manager_v1, ext_image_capture_source_v1,
        ext_output_image_capture_source_manager_v1,
    },
    image_copy_capture::v1::client::{
        ext_image_copy_capture_cursor_session_v1, ext_image_copy_capture_frame_v1,
        ext_image_copy_capture_manager_v1, ext_image_copy_capture_session_v1,
    },
};

use super::{
    CaptureCursorSession, CaptureFrame, CaptureSession, Rect, ScreencopyCursorSessionDataExt,
    ScreencopyFrameDataExt, ScreencopyHandler, ScreencopySessionDataExt, ScreencopyState,
};
use crate::GlobalData;

impl<D> Dispatch<ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1, GlobalData, D>
    for ScreencopyState
where
    D: Dispatch<ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1, GlobalData>
        + ScreencopyHandler,
{
    fn event(
        _: &mut D,
        _: &ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1,
        _: ext_image_copy_capture_manager_v1::Event,
        _: &GlobalData,
        _: &Connection,
        _: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D, U> Dispatch<ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1, U, D>
    for ScreencopyState
where
    D: Dispatch<ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1, U>
        + ScreencopyHandler,
    U: ScreencopySessionDataExt,
{
    fn event(
        app_data: &mut D,
        session: &ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1,
        event: ext_image_copy_capture_session_v1::Event,
        udata: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let formats = &udata.screencopy_session_data().formats;
        match event {
            ext_image_copy_capture_session_v1::Event::BufferSize { width, height } => {
                formats.lock().unwrap().buffer_size = (width, height);
            }
            ext_image_copy_capture_session_v1::Event::ShmFormat { format } => {
                if let WEnum::Value(value) = format {
                    formats.lock().unwrap().shm_formats.push(value);
                }
            }
            ext_image_copy_capture_session_v1::Event::DmabufDevice { device } => {
                let device = libc::dev_t::from_ne_bytes(device.try_into().unwrap());
                formats.lock().unwrap().dmabuf_device = Some(device);
            }
            ext_image_copy_capture_session_v1::Event::DmabufFormat { format, modifiers } => {
                let modifiers = modifiers
                    .chunks_exact(8)
                    .map(|x| u64::from_ne_bytes(x.try_into().unwrap()))
                    .collect();
                formats
                    .lock()
                    .unwrap()
                    .dmabuf_formats
                    .push((format, modifiers));
            }
            ext_image_copy_capture_session_v1::Event::Done => {
                if let Some(session) = udata
                    .screencopy_session_data()
                    .session
                    .get()
                    .unwrap()
                    .upgrade()
                    .map(CaptureSession)
                {
                    app_data.init_done(conn, qh, &session, &formats.lock().unwrap());
                }
            }
            ext_image_copy_capture_session_v1::Event::Stopped => {
                if let Some(session) = udata
                    .screencopy_session_data()
                    .session
                    .get()
                    .unwrap()
                    .upgrade()
                    .map(CaptureSession)
                {
                    app_data.stopped(conn, qh, &session);
                }
                session.destroy();
            }
            _ => unreachable!(),
        }
    }
}

impl<D, U> Dispatch<ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1, U, D>
    for ScreencopyState
where
    D: Dispatch<ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1, U> + ScreencopyHandler,
    U: ScreencopyFrameDataExt,
{
    fn event(
        app_data: &mut D,
        screencopy_frame: &ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1,
        event: ext_image_copy_capture_frame_v1::Event,
        udata: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        let frame = &udata.screencopy_frame_data().frame;
        match event {
            ext_image_copy_capture_frame_v1::Event::Transform { transform } => {
                frame.lock().unwrap().transform = transform;
            }
            ext_image_copy_capture_frame_v1::Event::Damage {
                x,
                y,
                width,
                height,
            } => {
                frame.lock().unwrap().damage.push(Rect {
                    x,
                    y,
                    width,
                    height,
                });
            }
            ext_image_copy_capture_frame_v1::Event::PresentationTime {
                tv_sec_hi,
                tv_sec_lo,
                tv_nsec,
            } => {
                let secs = (u64::from(tv_sec_hi) << 32) + u64::from(tv_sec_lo);
                let duration = Duration::new(secs, tv_nsec);
                frame.lock().unwrap().present_time = Some(duration);
            }
            ext_image_copy_capture_frame_v1::Event::Ready => {
                let frame = frame.lock().unwrap().clone();
                app_data.ready(
                    conn,
                    qh,
                    &CaptureFrame {
                        frame: screencopy_frame.clone(),
                    },
                    frame,
                );
                screencopy_frame.destroy();
            }
            ext_image_copy_capture_frame_v1::Event::Failed { reason } => {
                app_data.failed(
                    conn,
                    qh,
                    &CaptureFrame {
                        frame: screencopy_frame.clone(),
                    },
                    reason,
                );
                screencopy_frame.destroy();
            }
            _ => unreachable!(),
        }
    }
}

impl<D, U>
    Dispatch<ext_image_copy_capture_cursor_session_v1::ExtImageCopyCaptureCursorSessionV1, U, D>
    for ScreencopyState
where
    D: Dispatch<ext_image_copy_capture_cursor_session_v1::ExtImageCopyCaptureCursorSessionV1, U>
        + ScreencopyHandler,
    U: ScreencopyCursorSessionDataExt,
{
    fn event(
        app_data: &mut D,
        _screencopy_cursor_session: &ext_image_copy_capture_cursor_session_v1::ExtImageCopyCaptureCursorSessionV1,
        event: ext_image_copy_capture_cursor_session_v1::Event,
        udata: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        match event {
            ext_image_copy_capture_cursor_session_v1::Event::Enter => {
                if let Some(session) = udata
                    .screencopy_cursor_session_data()
                    .session
                    .get()
                    .unwrap()
                    .upgrade()
                    .map(CaptureCursorSession)
                {
                    app_data.cursor_enter(conn, qh, &session);
                }
            }
            ext_image_copy_capture_cursor_session_v1::Event::Leave => {
                if let Some(session) = udata
                    .screencopy_cursor_session_data()
                    .session
                    .get()
                    .unwrap()
                    .upgrade()
                    .map(CaptureCursorSession)
                {
                    app_data.cursor_leave(conn, qh, &session);
                }
            }
            ext_image_copy_capture_cursor_session_v1::Event::Position { x, y } => {
                if let Some(session) = udata
                    .screencopy_cursor_session_data()
                    .session
                    .get()
                    .unwrap()
                    .upgrade()
                    .map(CaptureCursorSession)
                {
                    app_data.cursor_position(conn, qh, &session, x, y);
                }
            }
            ext_image_copy_capture_cursor_session_v1::Event::Hotspot { x, y } => {
                if let Some(session) = udata
                    .screencopy_cursor_session_data()
                    .session
                    .get()
                    .unwrap()
                    .upgrade()
                    .map(CaptureCursorSession)
                {
                    app_data.cursor_hotspot(conn, qh, &session, x, y);
                }
            }
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch<ext_image_capture_source_v1::ExtImageCaptureSourceV1, GlobalData, D>
    for ScreencopyState
where
    D: Dispatch<ext_image_capture_source_v1::ExtImageCaptureSourceV1, GlobalData>
        + ScreencopyHandler,
{
    fn event(
        _app_data: &mut D,
        _source: &ext_image_capture_source_v1::ExtImageCaptureSourceV1,
        _event: ext_image_capture_source_v1::Event,
        _udata: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D>
    Dispatch<
        ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
        GlobalData,
        D,
    > for ScreencopyState
where
    D: Dispatch<
            ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
            GlobalData,
        > + ScreencopyHandler,
{
    fn event(
        _app_data: &mut D,
        _source: &ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
        _event: ext_output_image_capture_source_manager_v1::Event,
        _udata: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D>
    Dispatch<
        ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1,
        GlobalData,
        D,
    > for ScreencopyState
where
    D: Dispatch<
            ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1,
            GlobalData,
        > + ScreencopyHandler,
{
    fn event(
        _app_data: &mut D,
        _source: &ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1,
        _event: ext_foreign_toplevel_image_capture_source_manager_v1::Event,
        _udata: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

impl<D>
    Dispatch<
        zcosmic_workspace_image_capture_source_manager_v1::ZcosmicWorkspaceImageCaptureSourceManagerV1,
        GlobalData,
        D,
    > for ScreencopyState
where
    D: Dispatch<
            zcosmic_workspace_image_capture_source_manager_v1::ZcosmicWorkspaceImageCaptureSourceManagerV1,
            GlobalData,
        > + ScreencopyHandler,
{
    fn event(
        _app_data: &mut D,
        _source: &zcosmic_workspace_image_capture_source_manager_v1::ZcosmicWorkspaceImageCaptureSourceManagerV1,
        _event: zcosmic_workspace_image_capture_source_manager_v1::Event,
        _udata: &GlobalData,
        _conn: &Connection,
        _qh: &QueueHandle<D>,
    ) {
        unreachable!()
    }
}

#[macro_export]
macro_rules! delegate_screencopy {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::wayland_protocols::ext::image_capture_source::v1::client::ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1: $crate::GlobalData
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::wayland_protocols::ext::image_capture_source::v1::client::ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1: $crate::GlobalData
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::wayland_protocols::ext::image_capture_source::v1::client::ext_image_capture_source_v1::ExtImageCaptureSourceV1: $crate::GlobalData
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::cosmic_protocols::image_capture_source::v1::client::zcosmic_workspace_image_capture_source_manager_v1::ZcosmicWorkspaceImageCaptureSourceManagerV1: $crate::GlobalData
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1: $crate::GlobalData
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!(@<$( $lt $( : $clt $(+ $dlt )* )? ),* SessionData: ($crate::screencopy::ScreencopySessionDataExt)> $ty: [
            $crate::wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1: SessionData
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!(@<$( $lt $( : $clt $(+ $dlt )* )? ),* FrameData: ($crate::screencopy::ScreencopyFrameDataExt)> $ty: [
            $crate::wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1: FrameData
        ] => $crate::screencopy::ScreencopyState);
        $crate::wayland_client::delegate_dispatch!(@<$( $lt $( : $clt $(+ $dlt )* )? ),* CursorSessionData: ($crate::screencopy::ScreencopyCursorSessionDataExt)> $ty: [
            $crate::wayland_protocols::ext::image_copy_capture::v1::client::ext_image_copy_capture_cursor_session_v1::ExtImageCopyCaptureCursorSessionV1: CursorSessionData
        ] => $crate::screencopy::ScreencopyState);
    };
}
