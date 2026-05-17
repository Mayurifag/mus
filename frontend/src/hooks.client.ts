// Client-side hooks for SvelteKit application
// Service worker functionality has been removed

window.addEventListener("error", (event) => {
  console.error("Unhandled client error", event.error ?? event.message);
});

window.addEventListener("unhandledrejection", (event) => {
  console.error("Unhandled client promise rejection", event.reason);
});
