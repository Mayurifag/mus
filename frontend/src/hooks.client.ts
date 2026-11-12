window.addEventListener("error", (event) => {
  console.error("Unhandled client error", event.error ?? event.message);
});

window.addEventListener("unhandledrejection", (event) => {
  console.error("Unhandled client promise rejection", event.reason);
});
