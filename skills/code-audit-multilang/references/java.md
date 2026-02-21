# Java Audit Focus

## Frameworks To Detect

- Spring Boot/Spring Security, Struts, Jakarta EE, MyBatis, Hibernate.

## High-Risk Checks

- Broken method-level authorization (`@PreAuthorize` gaps).
- Insecure deserialization (`ObjectInputStream`, custom Jackson polymorphic typing).
- SQL injection through string-built native queries.
- Expression language injection (SpEL, template engines).
- SSRF in `RestTemplate/WebClient` with user-controlled URL.
- Path traversal in file controllers.
- XXE in XML parsers (unsafe factory settings).
- Sensitive actuator endpoints exposure.

## Quick Grep Targets

- `ObjectInputStream`
- `@PreAuthorize`, `@Secured`
- `createNativeQuery(`
- `Runtime.getRuntime().exec(`
- `DocumentBuilderFactory.newInstance(`
- `RestTemplate`, `WebClient.create(`

## Safe Patterns To Prefer

- Central authorization checks plus method-level enforcement.
- Disable polymorphic deserialization unless explicitly constrained.
- XML parser hardening: disable external entities/DTDs.
