# Analytics QA — November 28, 2026

## Verified

- 144 consistency checks against the deployed API across Staff, Reviewer, and all five categories. A temporary fictional workspace was created and removed after verification.
- Backend analytics integration tests: 2 passed, including a controlled 50% completeness fixture, readiness median, empty denominators, due-today boundaries, follow-up actions, ownership and workspace isolation.
- Frontend suite originally passed 17 tests, including 5 new analytics interaction tests covering metric and chart drill-downs, category reset, empty states, correct matter navigation, selected-record CSV export, formula/quote escaping, refresh recovery and session expiry.

The live API metrics reconciled with their returned record arrays. Category-specific matter counts summed to the overall totals, and child records remained within the selected matter set. No calculation or interaction defect was found in the tested paths.

## User workspace shown during manual QA

The user's edited workspace screenshots show the following values. These differ from a fresh sample workspace and should not be compared directly with the baseline below.

| Metric                         | Staff        | Reviewer      |
| ------------------------------ | ------------ | ------------- |
| Visible matters                | 7            | 13            |
| Active matters                 | 6            | 11            |
| Required document completeness | 45.0% (9/20) | 47.4% (18/38) |
| Outstanding requirements       | 11           | 20            |
| Overdue follow-ups             | 2            | 3             |
| Median time to first readiness | 5 days       | 5 days        |
| Readiness cohort               | 2 matters    | 4 matters     |

The percentages agree with their displayed numerators and denominators. The stage charts sum to the active counts (Staff 2+1+2+1=6; Reviewer 3+3+3+2=11). Age buckets sum to outstanding requirements (Staff 1+7+3=11; Reviewer 1+12+7=20). The screenshots alone do not verify the underlying records; inspect the corresponding drill-down rows to complete that check.

For edited workspaces, derive expected counts from the current register and drill-down records rather than requiring the fresh sample values. Added matters, document acceptance, follow-up actions, category edits, deletion and archiving can change the totals.

## Baseline for a fresh workspace

These values apply to the untouched sample workspace checked during this QA run. Existing visitor edits change the expected totals.

| Metric                         | Staff        | Reviewer      |
| ------------------------------ | ------------ | ------------- |
| Visible matters                | 6            | 12            |
| Active matters                 | 5            | 10            |
| Required document completeness | 44.4% (8/18) | 47.2% (17/36) |
| Outstanding requirements       | 10           | 19            |
| Overdue follow-ups             | 2            | 3             |
| Median time to first readiness | 5 days       | 5 days        |
| Readiness cohort               | 2 matters    | 4 matters     |

## Manual browser checks still to complete

The computer-use browser connection was unavailable, so this QA does not establish visual layout, actual browser download behavior, keyboard operation or mobile usability. The user has not yet confirmed manual analytics QA.

1. Open Analytics as Staff. For the untouched baseline, compare the cards with the table above. If retaining an edited session, compare the visible matter count with the register instead.
2. Click each metric card. Verify the underlying table changes. Active matters excludes Closed; overdue follow-ups excludes tasks due today and completed tasks; the readiness cohort excludes unfinished matters.
3. Click a stage bar, each requirement age bucket, and an assignee bar. Confirm the rows match the clicked label. Click a matter reference and confirm it opens that matter's checklist.
4. Change Service category. All cards, charts and rows should use that category, with the underlying table resetting to All visible matters. A Staff criminal-category baseline is empty and should show N/A for completeness and readiness median.
5. Select a specific drill-down, choose Export CSV, and open the downloaded file. Its rows should match the selected table and category. Check characters and columns in the spreadsheet application you use.
6. Complete one overdue follow-up in Matters, return to Analytics, and refresh if needed. The overdue count should fall by one; document completeness should not change. Reopening that overdue task should restore the count.
7. Record receipt of a missing required item. Completeness should stay the same. Accept it as Reviewer and return to Analytics: accepted required documents should rise by one. Switch roles within the same tab to preserve the workspace.
8. Verify CRUD effects: a new matter changes scope and denominator, an edited category moves it between category reports, a deleted unused draft disappears, and an archived matter leaves active/overdue counts while remaining in historical completeness and readiness cohorts.
9. Test narrow/mobile width and browser zoom. Cards, labels and controls should stay readable; tables may scroll horizontally without clipping the page. Tab through filters, cards, chart buttons, links and export; check visible focus.

Keep findings in terms of the action, expected result, actual result, role, selected category and whether the workspace was fresh or edited. Screenshots are useful for visual issues.
