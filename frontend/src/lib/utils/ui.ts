import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export type WithElementRef<T = Record<string, unknown>> = T & {
  ref?: HTMLElement | null;
};

export type WithoutChild<
  T extends Record<string, unknown> = Record<string, unknown>,
> = Omit<T, "child">;

export type WithoutChildrenOrChild<
  T extends Record<string, unknown> = Record<string, unknown>,
> = Omit<T, "children" | "child">;

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
