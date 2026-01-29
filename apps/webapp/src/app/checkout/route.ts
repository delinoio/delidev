import { Checkout } from "@polar-sh/nextjs";

const polarAccessToken = process.env.POLAR_ACCESS_TOKEN;

if (!polarAccessToken) {
  throw new Error(
    "Environment variable POLAR_ACCESS_TOKEN must be set for checkout."
  );
}

export const GET = Checkout({
  accessToken: polarAccessToken,
  successUrl:
    process.env.CHECKOUT_SUCCESS_URL || "https://deli.dev/thank-you",
  server: (process.env.POLAR_SERVER as "sandbox" | "production") || "sandbox",
});
