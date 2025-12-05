# Jogen Philosophy

## 1. The Trinity of Development

In modern software development, we rely on a fragmented ecosystem of tools: Git for code, Jira for tasks, and Confluence for knowledge. This separation creates friction and context switching.

**Jogen** unifies these three pillars into a single atomic unit.

When you start a Jogen **Project**, you are capturing:

-   **The Code** (Source files)
-   **The Plan** (Tasks and Board state)
-   **The Knowledge** (Documentation and Decisions)

All these aspects are stored together in a single Jogen project. This gives you a holistic view of your software. Viewing a snapshot from six months ago restores not just the code, but the *context*—showing you exactly what tasks were in progress and what documentation existed at that specific moment.

## 2. Immutable Truth (The Ledger)

History in Jogen is **Append-Only**.
Unlike Git, where history can be rewritten (`rebase`, `force-push`) to create a "clean" timeline, Jogen believes the history should reflect the reality of development.

-   **Safety:** You can never accidentally lose work.
-   **Trust:** The history is a verifiable sequence of events, similar to a blockchain.

*Note: For security leaks (e.g., passwords), Jogen provides a specific `redact` capability that shreds the data blob while preserving the historical graph integrity.*

## 3. Explicit Intent

A Version Control System should track *why* a change was made, not just *what* lines were edited.

-   **Context is Mandatory:** Every snapshot in Jogen requires a **Context Type** (e.g., `Feature`, `Fix`, `Refactor`).
-   **Task Linking:** Jogen encourages linking every snapshot to a native **Task**. This creates an automatic, semantic link between your planning and your execution.
