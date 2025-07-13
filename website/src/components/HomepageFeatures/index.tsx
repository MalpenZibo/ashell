import type { ReactNode } from "react";
import clsx from "clsx";
import Heading from "@theme/Heading";
import styles from "./styles.module.css";

type FeatureItem = {
  title: string;
  Svg: React.ComponentType<React.ComponentProps<"svg">>;
  description: ReactNode;
};

const FeatureList: FeatureItem[] = [
  {
    title: "Ready to Go",
    Svg: require("@site/static/img/rocket.svg").default,
    description: (
      <>
        Ashell is ready to use out of the box. Just install it, start using it,
        and customize only what you need.
      </>
    ),
  },
  {
    title: "Everything You Need, Built In",
    Svg: require("@site/static/img/settings.svg").default,
    description: (
      <>
        Ashell comes with essential modules like workspaces, time, battery,
        network, and more. No need to hunt for plugins or write custom scripts.
      </>
    ),
  },
  {
    title: "Powered by iced",
    Svg: require("@site/static/img/iced.svg").default,
    description: (
      <>
        A cross-platform GUI library for Rust focused on simplicity and
        type-safety.
      </>
    ),
  },
];

function Feature({ title, Svg, description }: FeatureItem) {
  return (
    <div className={clsx("col col--4")}>
      <div className="text--center">
        <Svg className={styles.featureSvg} role="img" />
      </div>
      <div className="text--center padding-horiz--md">
        <Heading as="h3">{title}</Heading>
        <p>{description}</p>
      </div>
    </div>
  );
}

export default function HomepageFeatures(): ReactNode {
  return (
    <section className={styles.features}>
      <div className="container">
        <div className="row">
          {FeatureList.map((props, idx) => (
            <Feature key={idx} {...props} />
          ))}
        </div>
      </div>
    </section>
  );
}
