# Documents

HTTP input models and handlers live in mod.rs. service.rs enforces receipt/review transitions and writes activity within a transaction. Shared matter locking, readiness, and authorization helpers live in matters/workflow.rs. See docs/workflow.md.
