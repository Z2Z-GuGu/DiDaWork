import React from "react";
import ReactDOM from "react-dom/client";

import App from "./App";
import "./index.css";
import { applyTheme, cachedTheme } from "./lib/theme";

/** 首帧之前先把明暗定好，避免深色偏好下闪一帧白底。 */
applyTheme(cachedTheme());

function render() {
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}

// 开发时若用浏览器直接打开（没有 Tauri 运行时），先注入一套假数据再渲染。
// 打包时 `import.meta.env.DEV` 是常量 false，这段会被整体移除。
if (import.meta.env.DEV) {
  void import("./dev/mock").then(
    (mock) => {
      mock.install();
      render();
    },
    render,
  );
} else {
  render();
}
