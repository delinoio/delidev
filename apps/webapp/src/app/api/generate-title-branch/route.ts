import { NextRequest, NextResponse } from "next/server";
import { Polar } from "@polar-sh/sdk";
import Redis from "ioredis";
import { POLAR_ORGANIZATION_ID } from "../constants";

// Initialize Polar SDK client
const polar = new Polar({
  accessToken: process.env.POLAR_ACCESS_TOKEN,
  server: (process.env.POLAR_SERVER as "sandbox" | "production") || "sandbox",
});

// Redis cache configuration
const CACHE_TTL_SECONDS = 5 * 60; // 5 minutes
const CACHE_KEY_PREFIX = "license:";

// Redis client singleton
let redisClient: Redis | null = null;

function getRedisClient(): Redis | null {
  if (!process.env.REDIS_URL) {
    console.warn("REDIS_URL not set, caching disabled");
    return null;
  }

  if (!redisClient) {
    redisClient = new Redis(process.env.REDIS_URL, {
      maxRetriesPerRequest: 3,
      connectTimeout: 5000,
      lazyConnect: true,
    });

    redisClient.on("error", (err) => {
      console.error("Redis connection error:", err);
    });
  }

  return redisClient;
}

interface GenerateRequest {
  prompt: string;
  licenseKey: string;
}

interface GenerateResponse {
  title: string;
  branchName: string;
}

/**
 * Validates a license key against Polar.sh API with Redis caching
 */
async function validateLicense(licenseKey: string): Promise<boolean> {
  const redis = getRedisClient();
  const cacheKey = `${CACHE_KEY_PREFIX}${licenseKey}`;

  // Check Redis cache first
  if (redis) {
    try {
      const cached = await redis.get(cacheKey);
      if (cached !== null) {
        return cached === "valid";
      }
    } catch (error) {
      console.error("Redis cache read error:", error);
      // Continue with API validation if cache fails
    }
  }

  try {
    const result = await polar.licenseKeys.validate({
      key: licenseKey,
      organizationId: POLAR_ORGANIZATION_ID,
    });

    const valid = result.status === "granted";

    // Cache the result in Redis
    if (redis) {
      try {
        await redis.setex(
          cacheKey,
          CACHE_TTL_SECONDS,
          valid ? "valid" : "invalid"
        );
      } catch (error) {
        console.error("Redis cache write error:", error);
      }
    }

    return valid;
  } catch (error) {
    // Cache invalid result on error (e.g., license not found)
    if (redis) {
      try {
        await redis.setex(cacheKey, CACHE_TTL_SECONDS, "invalid");
      } catch (cacheError) {
        console.error("Redis cache write error:", cacheError);
      }
    }
    console.error("Failed to validate license:", error);
    return false;
  }
}

/**
 * Generates a task title and branch name from a prompt using OpenRouter API
 */
async function generateTitleAndBranch(
  prompt: string
): Promise<GenerateResponse> {
  const openRouterApiKey = process.env.OPENROUTER_API_KEY;

  if (!openRouterApiKey) {
    throw new Error("OPENROUTER_API_KEY environment variable not set");
  }

  const systemPrompt = `You are a helpful assistant that generates concise task titles and git branch names from task descriptions.

Given a task prompt, respond with a JSON object containing:
1. "title": A concise, descriptive task title (max 80 characters). Should describe what the task accomplishes.
2. "branchName": A git branch name following these rules:
   - Use lowercase letters, numbers, and hyphens only
   - No spaces or special characters
   - Should be descriptive but concise (max 50 characters)
   - Use format like "feature/description" or "fix/description"

Respond ONLY with valid JSON, no markdown or explanation.`;

  const userPrompt = `Generate a task title and branch name for the following task description:

${prompt}`;

  // Add 30 second timeout for API call
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), 30000);

  try {
    const response = await fetch(
      "https://openrouter.ai/api/v1/chat/completions",
      {
        method: "POST",
        headers: {
          Authorization: `Bearer ${openRouterApiKey}`,
          "Content-Type": "application/json",
          "HTTP-Referer": "https://deli.dev",
          "X-Title": "DeliDev",
        },
        body: JSON.stringify({
          model: "anthropic/claude-haiku-4.5",
          messages: [
            { role: "system", content: systemPrompt },
            { role: "user", content: userPrompt },
            { role: "assistant", content: "{" },
          ],
          temperature: 0.3,
          max_tokens: 200,
        }),
        signal: controller.signal,
      }
    );

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`OpenRouter API error: ${error}`);
    }

    const data = await response.json();
    const content = data.choices?.[0]?.message?.content;

    if (!content) {
      throw new Error("No response from OpenRouter API");
    }

    try {
      // Parse the JSON response (prepend the prefilled "{" that was used to guide the model)
      const jsonString = "{" + content.trim();
      const parsed = JSON.parse(jsonString);
      return {
        title: parsed.title || "Untitled Task",
        branchName: parsed.branchName || "feature/new-task",
      };
    } catch {
      // If JSON parsing fails, try to extract from the response
      console.error("Failed to parse OpenRouter response:", content);
      throw new Error("Failed to parse AI response");
    }
  } catch (error) {
    if (error instanceof Error && error.name === "AbortError") {
      throw new Error("OpenRouter API request timed out");
    }
    throw error;
  } finally {
    clearTimeout(timeoutId);
  }
}

export async function POST(request: NextRequest) {
  try {
    const body: GenerateRequest = await request.json();
    const { prompt, licenseKey } = body;

    // Validate required fields
    if (!prompt || typeof prompt !== "string") {
      return NextResponse.json(
        { error: "Missing or invalid prompt" },
        { status: 400 }
      );
    }

    if (!licenseKey || typeof licenseKey !== "string") {
      return NextResponse.json(
        { error: "Missing or invalid license key" },
        { status: 400 }
      );
    }

    // Validate license
    const isValid = await validateLicense(licenseKey);
    if (!isValid) {
      return NextResponse.json(
        { error: "Invalid or expired license key" },
        { status: 401 }
      );
    }

    // Generate title and branch name
    const result = await generateTitleAndBranch(prompt);

    return NextResponse.json(result);
  } catch (error) {
    console.error("Error in generate-title-branch:", error);
    return NextResponse.json(
      {
        error:
          error instanceof Error ? error.message : "Internal server error",
      },
      { status: 500 }
    );
  }
}
