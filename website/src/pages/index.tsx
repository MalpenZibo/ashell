import type { ReactNode } from "react";
import clsx from "clsx";
import Link from "@docusaurus/Link";
import useDocusaurusContext from "@docusaurus/useDocusaurusContext";
import Layout from "@theme/Layout";
import HomepageFeatures from "@site/src/components/HomepageFeatures";
import HomepageHighlights from "@site/src/components/HomepageHighlights";

import styles from "./index.module.css";

function HomepageHeader() {
  const { siteConfig } = useDocusaurusContext();
  return (
    <header className={clsx("hero hero--primary", styles.mainBar)}>
      <div className={clsx("container", styles.container)}>
        <div className={clsx(styles.logo)}>
          <img src="./img/logo.svg" alt="Logo" className={styles.logoImg} />
          <img
            src="./img/ashell_text.svg"
            alt="Ashell"
            className={styles.logoText}
          />
        </div>
        <h1 className={clsx("hero__subtitle", styles.tagline)}>
          {siteConfig.tagline}
        </h1>
        <div className={styles.buttons}>
          <Link
            className="button button--secondary button--lg"
            to="/docs/intro"
          >
            Get Started
          </Link>
        </div>
      </div>
    </header>
  );
}

export default function Home(): ReactNode {
  return (
    <Layout
      title="Wayland status bar for Hyprland, Niri and more"
      description="Ashell is a ready to go status bar for Wayland compositors like Hyprland, Niri and MangoWC. Built-in workspaces, system tray, notifications, media player, system info, weather and quick settings. Written in Rust."
    >
      <HomepageHeader />
      <main>
        <HomepageFeatures />
        <HomepageHighlights />
      </main>
    </Layout>
  );
}
