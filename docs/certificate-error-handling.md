# Certificate Error Handling for Non-Standard Certificates

## Overview

Enhanced Sentinel AI's proxy functionality to handle websites with non-standard or problematic certificates. This addresses the issue where certificate validation failures (such as "CN不符合标准") would prevent traffic capture.

## Changes Made

### 1. Certificate Authority Enhancements

**File**: `src-tauri/sentinel-passive/src/certificate_authority.rs`

#### Improvements:
- **Hostname Sanitization**: Added `sanitize_hostname()` method to clean invalid characters from hostnames
- **Extended Certificate Fields**: 
  - Added `OrganizationName` to Distinguished Name for better compatibility
  - Enhanced Key Usage extensions (DigitalSignature, KeyEncipherment)
  - Added Extended Key Usage for both ServerAuth and ClientAuth
- **Wildcard Domain Support**: Automatically adds both wildcard and base domain to SAN
- **Fallback Handling**: Gracefully handles IA5String conversion failures with sanitized hostnames
- **Cipher Suite Flexibility**: Enabled `ignore_client_order` for weak cipher support

### 2. Proxy Server Certificate Verification

**File**: `src-tauri/sentinel-passive/src/proxy.rs`

#### Improvements:
- **SNI Configuration**: Explicitly enabled SNI for non-standard servers
- **Legacy Support**: Disabled secret extraction to support legacy renegotiation
- **Protocol Support**: Maintained ALPN for h2 and http/1.1 compatibility

### 3. Frontend User Experience

**File**: `src/components/passive/ProxyHistory.vue`

#### Features Added:
- **Certificate Error Dialog**: Modal dialog explaining certificate issues
- **Visual Indicators**: 
  - Error badge icon on status code 0 (TLS handshake failed)
  - Red background tint for failed requests
- **Quick Actions**:
  - Check CA installation button
  - View certificate details
  - Continue with capture despite errors
- **Click Behavior**: Clicking on certificate error entries opens detailed error dialog

### 4. Internationalization

**Files**: 
- `src/i18n/locales/passiveScan/zh.ts`
- `src/i18n/locales/passiveScan/en.ts`

#### New Translations Added:
```typescript
certificateError: {
  title: 'Certificate Error',
  message: 'The certificate for this website is non-standard or has issues',
  details: 'Certificate Details',
  commonIssues: {
    invalidCN: 'Invalid CN (Common Name) format',
    expired: 'Certificate expired',
    selfSigned: 'Self-signed certificate',
    untrusted: 'Certificate chain not trusted',
    hostnameMMismatch: 'Hostname mismatch',
    weakSignature: 'Weak signature algorithm'
  },
  actions: {
    trustCert: 'Trust Certificate',
    viewDetails: 'View Details',
    ignore: 'Continue',
    stop: 'Stop Capture'
  },
  tips: {
    installCA: 'Make sure Sentinel AI root CA is installed and trusted',
    checkCAInstallation: 'Check Certificate Installation',
    caNotTrusted: 'Root CA not trusted',
    serverCertIssue: 'Target server certificate has issues'
  }
}
```

## Technical Details

### Certificate Generation Process

1. **Hostname Validation**: Remove or replace non-ASCII characters
2. **SAN (Subject Alternative Name)**: Add both DNS names and IP addresses
3. **Certificate Chain**: Always send full chain (leaf + CA) to clients
4. **Key Usage**: Set appropriate extensions for TLS server/client auth
5. **Validity Period**: 1 year with 60-second backward offset for clock skew

### Error Handling Flow

```
1. Client connects → 2. TLS Handshake → 3. Certificate Validation
                                              ↓
                                          [Invalid Cert]
                                              ↓
4. Generate Proxy Cert → 5. Sanitize Hostname → 6. Add Fallback SAN
                                              ↓
7. Sign with CA → 8. Send Full Chain → 9. Client Accepts
```

### Status Code Meanings

- **Status 0**: TLS handshake failed (certificate error)
- **Status -1**: HTTPS CONNECT tunnel established, awaiting response
- **Status 2xx-5xx**: Normal HTTP response codes

## User Guide

### When Certificate Errors Occur

1. **Visual Indication**: The request row will have:
   - Red background tint
   - Status badge showing "TLS ERR"
   - Warning icon next to status

2. **Click to Investigate**: Click on the error entry to see:
   - Host and URL information
   - Error details
   - Common issues explanation

3. **Resolve Issues**:
   - **Option A**: Install and trust Sentinel AI root CA
   - **Option B**: Continue capturing (errors are logged but won't stop proxy)

### Check CA Installation

Click "Check Certificate Installation" button to:
- Verify if root CA is installed
- Get path to certificate file
- Open certificate location in file manager

## Supported Scenarios

✅ Self-signed certificates
✅ Expired certificates
✅ Certificates with invalid CN format
✅ Hostname mismatches
✅ Weak signature algorithms
✅ Untrusted certificate chains
✅ Non-ASCII hostnames
✅ Wildcard domains

## Known Limitations

- Some extremely old TLS 1.0/1.1 servers may still fail
- Certificate pinning applications cannot be intercepted
- Some corporate proxy environments may need additional configuration

## Testing

The changes have been validated against:
- Standard HTTPS websites (e.g., Google, GitHub)
- Self-signed certificate servers
- Expired certificate sites
- Invalid CN format certificates (as shown in the user's screenshot)

## Future Enhancements

- Certificate error statistics dashboard
- Automatic CA trust checking on startup
- Configurable certificate policies per domain
- Certificate export/import functionality

