use iced::Row;
use iced::alignment::Vertical;
use iced::{Element, Length};

use crate::modules::OnModulePress;

pub struct ModuleView<'a, Msg> {
    pub content: ModuleContent<'a, Msg>,
}

pub enum ModuleContent<'a, Msg> {
    Row(ModuleRow<'a, Msg>),
    Element(Element<'a, Msg>),
}

impl<'a, Msg> std::fmt::Debug for ModuleContent<'a, Msg> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModuleContent::Element(_) => f.write_str("Element(..)"),
            ModuleContent::Row(row) => f
                .debug_struct("Row")
                .field("children", &row.children.len())
                .field("spacing", &row.spacing)
                .field("align_y", &row.align_y)
                .finish(),
        }
    }
}

impl<'a, Msg: 'a> ModuleView<'a, Msg> {
    pub fn new(content: impl Into<ModuleContent<'a, Msg>>) -> Self {
        Self {
            content: content.into(),
        }
    }

    pub fn into_element(self) -> Element<'a, Msg> {
        match self.content {
            ModuleContent::Element(element) => element,
            ModuleContent::Row(row) => row.into_element(),
        }
    }

    pub fn map<NewMsg>(self, f: impl Fn(Msg) -> NewMsg + Clone + 'a) -> ModuleView<'a, NewMsg>
    where
        NewMsg: 'a,
    {
        ModuleView::new(match self.content {
            ModuleContent::Element(element) => ModuleContent::Element(element.map(f)),
            ModuleContent::Row(row) => ModuleContent::Row(row.map(f)),
        })
    }

    pub fn map_elements(self, f: impl Fn(Element<'a, Msg>) -> Element<'a, Msg> + Clone) -> Self {
        Self {
            content: match self.content {
                ModuleContent::Element(element) => ModuleContent::Element(f(element)),
                ModuleContent::Row(mut row) => {
                    row.children = row
                        .children
                        .into_iter()
                        .map(|child| f.clone()(child))
                        .collect();
                    ModuleContent::Row(row)
                }
            },
        }
    }
}

impl<'a, Msg> From<Element<'a, Msg>> for ModuleView<'a, Msg> {
    fn from(element: Element<'a, Msg>) -> Self {
        Self {
            content: ModuleContent::Element(element),
        }
    }
}

impl<'a, Msg> From<ModuleRow<'a, Msg>> for ModuleView<'a, Msg> {
    fn from(row: ModuleRow<'a, Msg>) -> Self {
        Self {
            content: ModuleContent::Row(row),
        }
    }
}

pub struct ModuleRow<'a, Msg> {
    pub children: Vec<Element<'a, Msg>>,
    pub spacing: f32,
    pub align_y: Vertical,
    pub height: Length,
}

impl<'a, Msg: 'a> ModuleRow<'a, Msg> {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            spacing: 0.0,
            align_y: Vertical::Top,
            height: Length::Shrink,
        }
    }

    pub fn with_children<I>(children: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Element<'a, Msg>>,
    {
        let children = children.into_iter();
        let mut row = Self::with_capacity(children.size_hint().0);

        row.children.extend(children.map(Into::into));

        row
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            children: Vec::with_capacity(capacity),
            spacing: 0.0,
            align_y: Vertical::Top,
            height: Length::Shrink,
        }
    }

    pub fn push(mut self, child: impl Into<Element<'a, Msg>>) -> Self {
        self.children.push(child.into());
        self
    }

    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn align_y(mut self, align: impl Into<Vertical>) -> Self {
        self.align_y = align.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    pub fn into_element(self) -> Element<'a, Msg> {
        Row::with_children(self.children)
            .height(self.height)
            .align_y(self.align_y)
            .spacing(self.spacing)
            .into()
    }

    pub fn map<NewMsg>(self, f: impl Fn(Msg) -> NewMsg + Clone + 'a) -> ModuleRow<'a, NewMsg>
    where
        NewMsg: 'a,
    {
        ModuleRow {
            children: self
                .children
                .into_iter()
                .map(|child| child.map(f.clone()))
                .collect(),
            spacing: self.spacing,
            align_y: self.align_y,
            height: self.height,
        }
    }
}

pub struct ModuleResult<'a, Msg: 'static> {
    pub view: ModuleView<'a, Msg>,
    pub action: Option<OnModulePress>,
}
