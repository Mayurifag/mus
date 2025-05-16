import { dev } from "$app/environment";

// Register service worker
if ("serviceWorker" in navigator) {
  addEventListener("load", () => {
    navigator.serviceWorker
      .register("/service-worker.js", {
        type: dev ? "module" : "classic",
      })
      .then((registration) => {
        console.log("Service worker registered:", registration);
      })
      .catch((error) => {
        console.error("Service worker registration failed:", error);
      });
  });
}
