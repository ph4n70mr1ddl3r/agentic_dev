# Dynamics 365 Financials Reference

_Authored by the **Domain Modeler** hat via `forge`; schema-validated._

**Area:** financials  
**Source:** Microsoft Dynamics 365 Finance public documentation (model-reconstructed)

Model-reconstructed digest of the core concepts of Dynamics 365 Finance.

## Entities

### Company — Legal Entity _(master)_

A legal entity that owns financial transactions and a chart of accounts.

| Field | Type | Required | Description |
|---|---|---|---|
| `name` | text | yes |  |
| `baseCurrency` | text | yes |  |

### LedgerAccount — Ledger Account _(master)_

A chart of accounts entry representing a financial account.

| Field | Type | Required | Description |
|---|---|---|---|
| `accountNumber` | text | yes |  |
| `name` | text | yes |  |
| `type` | picklist | yes |  |

### JournalEntry — Journal Entry _(transactional)_

The header of a double-entry accounting journal.

| Field | Type | Required | Description |
|---|---|---|---|
| `entryNo` | text | yes |  |
| `entryDate` | date | yes |  |
| `companyId` | lookup | yes |  |
| `status` | picklist | yes |  |

**Relationships:** `lines` → JournalLine (one-to-many)

### JournalLine — Journal Line _(transactional)_

A single debit or credit line within a journal entry.

| Field | Type | Required | Description |
|---|---|---|---|
| `lineNumber` | integer | yes |  |
| `accountId` | lookup | yes |  |
| `debitAmount` | money |  |  |
| `creditAmount` | money |  |  |
| `currencyCode` | text | yes |  |

**Relationships:** `journalEntry` → JournalEntry (many-to-one)

### Customer — Customer _(master)_

A party that owes money to the organization.

| Field | Type | Required | Description |
|---|---|---|---|
| `customerAccount` | text | yes |  |
| `name` | text | yes |  |
| `currencyCode` | text | yes |  |

### Vendor — Vendor _(master)_

A party to whom the organization owes money.

| Field | Type | Required | Description |
|---|---|---|---|
| `vendorAccount` | text | yes |  |
| `name` | text | yes |  |
| `currencyCode` | text | yes |  |

### Invoice — Invoice _(transactional)_

A document requesting payment for goods or services.

| Field | Type | Required | Description |
|---|---|---|---|
| `invoiceNumber` | text | yes |  |
| `invoiceDate` | date | yes |  |
| `dueDate` | date | yes |  |
| `totalAmount` | money | yes |  |
| `status` | picklist | yes |  |

**Relationships:** `customer` → Customer (many-to-one); `vendor` → Vendor (many-to-one)

### Payment — Payment _(transactional)_

A transaction that settles an invoice.

| Field | Type | Required | Description |
|---|---|---|---|
| `paymentId` | text | yes |  |
| `paymentDate` | date | yes |  |
| `amount` | money | yes |  |
| `currencyCode` | text | yes |  |

**Relationships:** `invoice` → Invoice (many-to-one)

### Budget — Budget _(master)_

A financial plan for a period.

| Field | Type | Required | Description |
|---|---|---|---|
| `budgetId` | text | yes |  |
| `fiscalYear` | integer | yes |  |
| `status` | picklist | yes |  |

### FixedAsset — Fixed Asset _(master)_

A long-term tangible asset.

| Field | Type | Required | Description |
|---|---|---|---|
| `assetNumber` | text | yes |  |
| `name` | text | yes |  |
| `acquisitionCost` | money | yes |  |
| `depreciationMethod` | picklist | yes |  |

## Processes

### JournalPosting — Journal Posting

Validate and post a journal entry to the general ledger.

1. Create journal entry header
2. Add journal lines with debit/credit amounts
3. Validate that debits equal credits
4. Post the journal entry
5. Update ledger account balances

### InvoicePayment — Invoice Payment

Record a payment against an invoice and settle it.

1. Receive payment from customer or make payment to vendor
2. Create payment record
3. Apply payment to open invoice
4. Mark invoice as paid if fully settled
5. Post payment to general ledger

### BudgetPlanning — Budget Planning

Create and approve a budget for a fiscal year.

1. Define budget plan template
2. Enter budget amounts by account and period
3. Review and route for approval
4. Approve budget plan
5. Transfer to budget register entries

### FixedAssetDepreciation — Fixed Asset Depreciation

Calculate and post depreciation for fixed assets.

1. Run depreciation proposal
2. Review depreciation amounts
3. Post depreciation journal entry
4. Update asset book value

## Rules

- **BalancedEntry** _(error, before-post)_ — A journal entry's debits must equal its credits before posting.
- **InvoiceTotalMatch** _(error, before-post)_ — The sum of line amounts on an invoice must equal the total amount.
- **PaymentNotExceedInvoice** _(error, before-create)_ — A payment amount cannot exceed the remaining balance of the invoice.
- **BudgetPeriodValid** _(error, before-create)_ — Budget periods must fall within the fiscal year.
- **DepreciationMethodRequired** _(error, before-create)_ — A fixed asset must have a depreciation method assigned.
