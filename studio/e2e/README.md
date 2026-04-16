# E2E Testing Framework: Service Design & Behavior-Driven

This framework is built using **Playwright** and follows a **Human-Driven Design (HDD)** philosophy. Instead of testing technical implementation details, we test **User Journeys** and **Service Outcomes**.

## Core Principles

1.  **Behavior-Driven Requirements:** Tests are structured around "User Stories" and "Scenarios" (Given/When/Then).
2.  **Service Design Perspective:** We evaluate the end-to-end experience for three primary personas:
    *   **Caseworkers:** Efficiency, triage speed, and AI-assisted decision making.
    *   **Admins:** Agility in workflow design and system configuration.
    *   **Applicants:** Transparency, trust, and clarity in the determination process.
3.  **Page Object Model (POM):** UI interactions are abstracted into Page Objects (`e2e/pages/`) to keep tests readable as "Business Logic".

## Structure

- `e2e/journeys/`: Contains the actual test specifications organized by persona/journey.
- `e2e/pages/`: Contains Page Objects that encapsulate UI selectors and common actions.
- `playwright.config.ts`: Main configuration for the testing environment.

## Running Tests

To run the full suite:
```bash
npm run test:e2e
```

To run with UI mode (recommended for local development):
```bash
npx playwright test --ui
```

## Key Journeys Covered

- **Efficient Triage:** Verifies caseworkers can use "Quick Peek" to triage high-priority cases without context switching.
- **Workflow Evolution:** Verifies admins can navigate complex workflows using "Mini-map" and "Search".
- **Transparency & Trust:** Verifies applicants can inspect the evidence behind their decisions.
