import type { ReactNode } from "react";
import Heading from "@theme/Heading";
import styles from "./styles.module.css";

type HighlightItem = {
  title: string;
  description: ReactNode;
};

const HighlightList: HighlightItem[] = [
  {
    title: "Works with your compositor",
    description: (
      <>
        Dedicated integrations for <strong>Hyprland</strong>,{" "}
        <strong>Niri</strong> and <strong>MangoWC</strong>, plus a generic
        Wayland fallback that runs on any compositor supporting the standard
        layer-shell protocol.
      </>
    ),
  },
  {
    title: "Everything a bar needs",
    description: (
      <>
        Workspaces, active window, system tray, notifications with toasts,
        media player controls, system stats (CPU, RAM, temperature, disk,
        network speed), clock with weather, privacy indicators, and quick
        settings for audio, brightness, Wi-Fi, Bluetooth and power.
      </>
    ),
  },
  {
    title: "Extend it with shell commands",
    description: (
      <>
        Custom modules turn any script into a bar widget: buttons, text,
        icons and alert badges driven by JSON output. No plugin API to learn,
        no extra daemon to run.
      </>
    ),
  },
];

function Highlight({ title, description }: HighlightItem) {
  return (
    <div className="col col--4">
      <div className="text--center padding-horiz--md">
        <Heading as="h3">{title}</Heading>
        <p>{description}</p>
      </div>
    </div>
  );
}

export default function HomepageHighlights(): ReactNode {
  return (
    <section className={styles.highlights}>
      <div className="container">
        <Heading as="h2" className={styles.heading}>
          A complete status bar for your Wayland desktop
        </Heading>
        <div className="row">
          {HighlightList.map((props, idx) => (
            <Highlight key={idx} {...props} />
          ))}
        </div>
      </div>
    </section>
  );
}
