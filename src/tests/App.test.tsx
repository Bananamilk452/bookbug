import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import App from "../App";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("App", () => {
  it("renders the heading", () => {
    render(<App />);
    expect(screen.getByText("Welcome to Tauri + React")).toBeInTheDocument();
  });
});
