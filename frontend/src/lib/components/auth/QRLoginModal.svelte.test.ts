import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/svelte";
import QRLoginModal from "./QRLoginModal.svelte";

// Mock @castlenine/svelte-qrcode component
vi.mock("@castlenine/svelte-qrcode", () => ({
  default: vi.fn().mockImplementation((props) => ({
    $$: {
      component: "MockQRCode",
      props,
    },
  })),
}));

// Mock import.meta.env
vi.mock("$app/environment", () => ({
  browser: true,
}));

// Mock import.meta.env properly
Object.defineProperty(import.meta, "env", {
  value: {
    VITE_PUBLIC_API_HOST: "http://localhost:8000",
  },
  configurable: true,
});

// Mock localStorage
const mockLocalStorage = {
  getItem: vi.fn(),
  setItem: vi.fn(),
  removeItem: vi.fn(),
  clear: vi.fn(),
};

Object.defineProperty(window, "localStorage", {
  value: mockLocalStorage,
  writable: true,
});

describe("QRLoginModal", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Mock the environment variable for each test
    vi.stubEnv("VITE_PUBLIC_API_HOST", "http://localhost:8000");
  });

  it("should render when open is true", () => {
    mockLocalStorage.getItem.mockReturnValue("test-secret-key");

    render(QRLoginModal, {
      props: {
        open: true,
      },
    });

    expect(screen.getByText("Login on Mobile")).toBeInTheDocument();
  });

  it("should display QR code when auth token is available in localStorage", () => {
    mockLocalStorage.getItem.mockReturnValue("test-secret-key");

    render(QRLoginModal, {
      props: {
        open: true,
      },
    });

    const expectedUrl =
      "http://localhost:8000/api/v1/auth/login-by-secret/test-secret-key";
    expect(screen.getByText(expectedUrl)).toBeInTheDocument();
  });

  it("should show no authentication key message when no auth token in localStorage", () => {
    mockLocalStorage.getItem.mockReturnValue(null);

    render(QRLoginModal, {
      props: {
        open: true,
      },
    });

    expect(
      screen.getByText("No authentication key available"),
    ).toBeInTheDocument();
  });

  it("should display the full URL as selectable text", () => {
    mockLocalStorage.getItem.mockReturnValue("test-secret-key");

    render(QRLoginModal, {
      props: {
        open: true,
      },
    });

    const codeElement = screen.getByText(
      "http://localhost:8000/api/v1/auth/login-by-secret/test-secret-key",
    );
    expect(codeElement).toHaveClass("select-all");
  });
});
