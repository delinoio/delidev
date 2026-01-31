# iOS TestFlight Setup Guide

This document describes how to set up iOS TestFlight builds for DeliDev.

## Prerequisites

1. **Apple Developer Account** - An active Apple Developer Program membership ($99/year)
2. **App Store Connect Access** - Your Apple ID must have access to App Store Connect
3. **Xcode** - Required for local development and generating certificates

## Required GitHub Secrets

The following secrets must be configured in the GitHub repository settings:

### Code Signing Secrets

| Secret Name | Description |
|-------------|-------------|
| `APPLE_CERTIFICATE_BASE64` | Base64-encoded iOS Distribution certificate (.p12 file) |
| `APPLE_CERTIFICATE_PASSWORD` | Password for the .p12 certificate |
| `APPLE_PROVISIONING_PROFILE_BASE64` | Base64-encoded provisioning profile (.mobileprovision) |
| `APPLE_TEAM_ID` | Your Apple Developer Team ID (10 characters, e.g., `ABC123XYZ0`) |

### App Store Connect API Secrets

| Secret Name | Description |
|-------------|-------------|
| `APP_STORE_CONNECT_API_KEY_ID` | API Key ID from App Store Connect |
| `APP_STORE_CONNECT_API_ISSUER_ID` | Issuer ID from App Store Connect |
| `APP_STORE_CONNECT_API_KEY_BASE64` | Base64-encoded API private key (.p8 file) |

## Setup Steps

### 1. Create App ID in Apple Developer Portal

1. Go to [Apple Developer - Identifiers](https://developer.apple.com/account/resources/identifiers/list)
2. Click "+" to create a new identifier
3. Select "App IDs" → "App"
4. Enter:
   - Description: `DeliDev`
   - Bundle ID: `com.delidev.app` (must match `identifier` in `tauri.conf.json`)
5. Enable required capabilities (if any)
6. Click "Register"

### 2. Create Distribution Certificate

1. Open **Keychain Access** on your Mac
2. Go to **Keychain Access → Certificate Assistant → Request a Certificate From a Certificate Authority**
3. Enter your email and name, select "Saved to disk"
4. Go to [Apple Developer - Certificates](https://developer.apple.com/account/resources/certificates/list)
5. Click "+" → Select "Apple Distribution"
6. Upload the CSR file you created
7. Download the certificate and double-click to install
8. In Keychain Access, find the certificate, right-click → "Export..."
9. Save as .p12 with a password
10. Base64 encode it:
   ```bash
   base64 -i Certificates.p12 | pbcopy
   ```
11. Add to GitHub as `APPLE_CERTIFICATE_BASE64`
12. Add the password as `APPLE_CERTIFICATE_PASSWORD`

### 3. Create Provisioning Profile

1. Go to [Apple Developer - Profiles](https://developer.apple.com/account/resources/profiles/list)
2. Click "+" → Select "App Store Connect"
3. Select your App ID (`com.delidev.app`)
4. Select your distribution certificate
5. Name it (e.g., `DeliDev App Store`)
6. Download the profile
7. Base64 encode it:
   ```bash
   base64 -i DeliDev_App_Store.mobileprovision | pbcopy
   ```
8. Add to GitHub as `APPLE_PROVISIONING_PROFILE_BASE64`

### 4. Get Team ID

1. Go to [Apple Developer - Membership](https://developer.apple.com/account/#!/membership)
2. Find your Team ID (10 characters)
3. Add to GitHub as `APPLE_TEAM_ID`

### 5. Create App Store Connect API Key

1. Go to [App Store Connect - Users and Access - Keys](https://appstoreconnect.apple.com/access/api)
2. Click "+" to generate a new key
3. Name: `DeliDev CI`
4. Access: `App Manager` (or higher)
5. Click "Generate"
6. **Download the .p8 file immediately** (you can only download it once!)
7. Note the **Key ID** and **Issuer ID** shown on the page
8. Base64 encode the key:
   ```bash
   base64 -i AuthKey_XXXXXXXXXX.p8 | pbcopy
   ```
9. Add to GitHub:
   - `APP_STORE_CONNECT_API_KEY_ID` - The Key ID
   - `APP_STORE_CONNECT_API_ISSUER_ID` - The Issuer ID
   - `APP_STORE_CONNECT_API_KEY_BASE64` - The base64-encoded .p8 file

### 6. Create App in App Store Connect

1. Go to [App Store Connect - Apps](https://appstoreconnect.apple.com/apps)
2. Click "+" → "New App"
3. Fill in:
   - Platform: iOS
   - Name: `DeliDev`
   - Primary Language: English (or your preference)
   - Bundle ID: Select `com.delidev.app`
   - SKU: `com.delidev.app` (or any unique identifier)
4. Click "Create"

## Workflow Usage

### Automatic Builds

iOS builds are automatically triggered when pushing a tag:

```bash
git tag v0.0.5
git push origin v0.0.5
```

### Manual Builds

You can manually trigger a build from GitHub Actions:

1. Go to Actions → "iOS TestFlight"
2. Click "Run workflow"
3. Optionally check "Skip TestFlight upload" for build-only runs

### Dry Run

To test the build without uploading to TestFlight:

1. Trigger the workflow manually
2. Check "Skip TestFlight upload (build only)"

## Important Notes

- **Server Mode Only**: The iOS app only supports server-worker-client mode. Users must connect to a DeliDev server.
- **Bundle Identifier**: Must be `com.delidev.app` everywhere (App Store Connect, provisioning profile, `tauri.conf.json`)
- **First Upload**: The first upload to App Store Connect may take longer as Apple processes the new app

## Troubleshooting

### "No matching provisioning profiles found"

Ensure the provisioning profile:
1. Uses the correct App ID (`com.delidev.app`)
2. Uses the same distribution certificate
3. Is not expired

### "Unable to authenticate with App Store Connect"

Verify:
1. API key has correct permissions
2. Key ID and Issuer ID are correct
3. The .p8 file was base64-encoded correctly

### Build fails on Rust compilation

Ensure the iOS target is installed:
```bash
rustup target add aarch64-apple-ios
```
