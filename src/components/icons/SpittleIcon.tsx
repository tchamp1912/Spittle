const SpittleIcon = ({
  width = 24,
  height = 24,
}: {
  width?: number | string;
  height?: number | string;
}) => (
  <div
    style={{
      width: typeof width === "number" ? `${width}px` : width,
      height: typeof height === "number" ? `${height}px` : height,
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      fontSize: "1.5em",
    }}
  >
    💧
  </div>
);

export default SpittleIcon;
