declare module "@castlenine/svelte-qrcode" {
  import type { SvelteComponent } from "svelte";

  interface QRCodeProps {
    data: string;
    size?: number;
    backgroundColor?: string;
    color?: string;
    errorCorrectionLevel?: "L" | "M" | "Q" | "H";
    padding?: number;
    typeNumber?: number;
    modulesColor?: string;
    anchorsOuterColor?: string;
    anchorsInnerColor?: string;
    shape?: "square" | "circle";
    haveBackgroundRoundedEdges?: boolean;
    haveGappedModules?: boolean;
    isJoin?: boolean;
    isResponsive?: boolean;
    width?: number;
    height?: number;
    logoInBase64?: string;
    logoPath?: string;
    logoBackgroundColor?: string;
    logoPadding?: number;
    logoSize?: number;
    logoWidth?: number;
    logoHeight?: number;
    waitForLogo?: boolean;
    dispatchDownloadUrl?: boolean;
    downloadUrlFileFormat?: "svg" | "png" | "jpg" | "jpeg" | "webp";
  }

  export default class QRCode extends SvelteComponent<QRCodeProps> {}
}
