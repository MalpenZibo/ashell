import { ReactNode, useState } from "react";
import Layout from "@theme/Layout";
import clsx from "clsx";

import classes from "./gallery.module.css";

const importAll = (r) => r.keys().map(r);
const images = importAll(
  require.context("../../static/img/gallery", false, /\.(png|jpe?g|svg)$/),
).map((img) => img.default);

const maxThumbnail = 8;
const scrollLimit = 3;

export default function Gallery(): ReactNode {
  return (
    <Layout
      title={`Gallery`}
      description="A collection of images showcasing Ashell's features and appearance."
    >
      <ImageGallery items={images} />
    </Layout>
  );
}

function ImageGallery({ items }: { items: string[] }): ReactNode {
  const [selected, setSelected] = useState(0);
  const [[start, end], setSlices] = useState([0, maxThumbnail]);

  const changeIndex = (index: number) => {
    setSelected(index);

    if (maxThumbnail - index < scrollLimit) {
      const newStart = Math.max(
        0,
        start + scrollLimit - (maxThumbnail - index),
      );
      const newEnd = Math.min(items.length, newStart + maxThumbnail);

      const clampStart = Math.min(newEnd - maxThumbnail, newStart);

      setSelected(start - clampStart + index);
      setSlices([clampStart, newEnd]);
    }

    if (index + start < start + scrollLimit - 1) {
      const newStart = Math.max(0, start - (scrollLimit - 1 - index));
      const newEnd = Math.min(items.length, newStart + maxThumbnail);

      setSelected(start - newStart + index);
      setSlices([newStart, newEnd]);
    }
  };

  const diplayedImages = images.slice(start, end);

  return (
    <div className={classes.gallery}>
      <div className={classes.galleryContent}>
        <LeftArrow
          onClick={() => {
            if (selected > 0) {
              changeIndex(selected - 1);
            } else {
              setSlices([
                Math.max(0, items.length - maxThumbnail),
                items.length,
              ]);
              setSelected(maxThumbnail - 1);
            }
          }}
        />
        <div className={classes.selectedImage}>
          <img src={diplayedImages[selected]} alt="Selected" />
        </div>
        <RightArrow
          onClick={() => {
            if (selected < diplayedImages.length - 1) {
              changeIndex(selected + 1);
            } else {
              setSlices([0, maxThumbnail]);
              setSelected(0);
            }
          }}
        />
      </div>
      <div className={classes.thumbnailContainer}>
        {diplayedImages.map((src, index) => (
          <div
            className={clsx(classes.imageThumbnail, {
              [classes.selected]: index === selected,
              [classes.left]:
                images.length > maxThumbnail && index === 0 && start > 0,
              [classes.right]:
                images.length > maxThumbnail &&
                index === maxThumbnail - 1 &&
                end < items.length,
            })}
            key={index}
            onClick={() => {
              changeIndex(index);
            }}
          >
            <img src={src} alt={`Gallery image ${index + 1}`} />
          </div>
        ))}
      </div>
    </div>
  );
}

function LeftArrow({ onClick }: { onClick: () => void }): ReactNode {
  return (
    <div className={clsx(classes.arrow, classes.left)} onClick={onClick}>
      <svg viewBox="0 0 24 24" x="0" y="0">
        <path
          fill="none"
          stroke="currentColor"
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="m14 7l-5 5m0 0l5 5"
        />
      </svg>
    </div>
  );
}

function RightArrow({ onClick }: { onClick: () => void }): ReactNode {
  return (
    <div className={clsx(classes.arrow, classes.right)} onClick={onClick}>
      <svg viewBox="0 0 24 24" x="0" y="0">
        <path
          fill="none"
          stroke="currentColor"
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="m10 17l5-5m0 0l-5-5"
        />
      </svg>
    </div>
  );
}
