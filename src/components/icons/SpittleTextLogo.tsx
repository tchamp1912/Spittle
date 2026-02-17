import React from "react";

const SpittleTextLogo = ({
  width,
  height,
  className,
}: {
  width?: number | string;
  height?: number | string;
  className?: string;
}) => {
  return (
    <div
      className={className}
      style={{
        width: typeof width === "number" ? `${width}px` : width || "120px",
        height: typeof height === "number" ? `${height}px` : height || "40px",
        display: "flex",
        alignItems: "center",
        gap: "8px",
        fontSize: "18px",
        fontWeight: "bold",
        color: "var(--color-logo-primary)",
      }}
    >
      <span style={{ fontSize: "24px" }}>💧</span>
      <span>Spittle</span>
    </div>
  );
};

export default SpittleTextLogo;
