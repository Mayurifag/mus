<script module lang="ts">
  type Priority = "high" | "low" | "auto";
  const coverRequests = new WeakMap<Element, (priority: Priority) => void>();
  let coverObserver: IntersectionObserver | undefined;

  function observeCover(node: Element, request: (priority: Priority) => void) {
    if (typeof IntersectionObserver === "undefined") {
      request("auto");
      return;
    }

    const observer = (coverObserver ??= new IntersectionObserver(
      (entries, observer) => {
        const footerTop = document
          .querySelector("[data-player-footer]")
          ?.getBoundingClientRect().top;
        const bottom = Math.min(window.innerHeight, footerTop ?? Infinity);
        const buffered = [];
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          const start = coverRequests.get(entry.target);
          coverRequests.delete(entry.target);
          observer.unobserve(entry.target);
          if (!start) continue;
          const { top, bottom: coverBottom } = entry.boundingClientRect;
          if (top < bottom && coverBottom > 0) start("high");
          else buffered.push(start);
        }
        for (const start of buffered) start("low");
      },
      // The fixed footer already adds a lookahead buffer below the visible list.
      { rootMargin: "180px 0px 0px" },
    ));
    coverRequests.set(node, request);
    observer.observe(node);

    return {
      destroy() {
        coverRequests.delete(node);
        observer.unobserve(node);
      },
    };
  }
</script>

<script lang="ts">
  import { thumbHashToDataURL } from "thumbhash";

  let {
    url,
    thumbhash,
    title,
  }: {
    url: string | null;
    thumbhash?: string;
    title: string;
  } = $props();

  let priority = $state<Priority>();
  let image = $state<HTMLImageElement>();
  let status = $state<"loaded" | "failed">();
  let preview = $state<string>();
  const fallback = $derived(!url || status === "failed");

  function startWhenNear(node: Element) {
    if (url) {
      return observeCover(node, (nextPriority) => {
        priority = nextPriority;
      });
    }
  }

  $effect(() => {
    if (!priority || !thumbhash || !image || status) return;
    const target = image;
    const hash = thumbhash;
    // Let memory- and disk-cached images paint before decoding a preview.
    const timeout = setTimeout(() => {
      if (target.complete && target.naturalWidth > 0) return;
      try {
        preview = thumbHashToDataURL(
          Uint8Array.from(atob(hash), (c) => c.charCodeAt(0)),
        );
      } catch {
        // Keep the plain placeholder if the optional preview is invalid.
      }
    }, 100);
    return () => clearTimeout(timeout);
  });
</script>

<span
  use:startWhenNear
  class="bg-muted/30 h-14 w-14 shrink-0 overflow-hidden rounded-md bg-cover bg-center"
  style:background-image={preview ? `url("${preview}")` : undefined}
>
  <img
    bind:this={image}
    fetchpriority={priority}
    src={fallback ? "/images/no-cover.svg" : priority ? url : undefined}
    alt={fallback ? "No Album Cover" : `Album art for ${title}`}
    width="56"
    height="56"
    loading={url ? "eager" : "lazy"}
    decoding="async"
    class="h-full w-full object-cover"
    class:opacity-0={!fallback && !status}
    onload={() => {
      if (!fallback) {
        status = "loaded";
        preview = undefined;
      }
    }}
    onerror={() => {
      if (!fallback && priority) {
        status = "failed";
        preview = undefined;
      }
    }}
  />
</span>
