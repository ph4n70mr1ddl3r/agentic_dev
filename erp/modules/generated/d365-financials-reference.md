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

### LedgerAccount — Main Account _(master)_

A single account in the chart of accounts used for posting.

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
| `lineNo` | integer | yes |  |
| `accountId` | lookup | yes |  |
| `debitAmount` | money |  |  |
| `creditAmount` | money |  |  |
| `description` | text |  |  |

**Relationships:** `journalEntry` → JournalEntry (many-to-one); `ledgerAccount` → LedgerAccount (many-to-one)

### Vendor — Vendor _(master)_

A supplier from whom goods or services are procured.

| Field | Type | Required | Description |
|---|---|---|---|
| `vendorAccountNumber` | text | yes |  |
| `name` | text | yes |  |
| `currencyId` | lookup |  |  |

### Customer — Customer _(master)_

A buyer of goods or services.

| Field | Type | Required | Description |
|---|---|---|---|
| `customerAccountNumber` | text | yes |  |
| `name` | text | yes |  |
| `currencyId` | lookup |  |  |

### Invoice — Invoice _(transactional)_

A document requesting payment for goods or services.

| Field | Type | Required | Description |
|---|---|---|---|
| `invoiceNumber` | text | yes |  |
| `invoiceDate` | date | yes |  |
| `dueDate` | date | yes |  |
| `totalAmount` | money | yes |  |
| `status` | picklist | yes |  |

**Relationships:** `vendor` → Vendor (many-to-one); `customer` → Customer (many-to-one)

### Payment — Payment _(transactional)_

A payment made or received.

| Field | Type | Required | Description |
|---|---|---|---|
| `paymentId` | text | yes |  |
| `paymentDate` | date | yes |  |
| `amount` | money | yes |  |
| `direction` | picklist | yes |  |

**Relationships:** `invoice` → Invoice (many-to-one)

### Budget — Budget _(reference)_

A financial plan for a period.

| Field | Type | Required | Description |
|---|---|---|---|
| `budgetId` | text | yes |  |
| `fiscalYear` | integer | yes |  |
| `amount` | money | yes |  |

## Processes

### JournalPosting — Journal Posting

Validate and post a journal entry to the general ledger.

1. Create or open a journal entry
2. Enter lines with debit/credit amounts
3. Validate the entry balances
4. Post to the ledger
5. Update account balances

### InvoicePayment — Invoice Payment

Record a payment against an invoice and settle the open amount.

1. Receive payment from customer or make payment to vendor
2. Create a payment record
3. Apply payment to the invoice
4. Update invoice status to paid or partially paid
5. Post the payment to the general ledger

### BudgetPlanning — Budget Planning

Create and approve budgets for a fiscal period.

1. Define budget plan template
2. Enter budget amounts by account
3. Review and route for approval
4. Approve budget
5. Transfer to budget register

## Rules

- **BalancedEntry** _(error, before-post)_ — A journal entry's debits must equal its credits before posting.
- **MandatoryAccountType** _(error, before-post)_ — Each journal line must reference a valid ledger account of the correct type.
- **InvoiceTotalPositive** _(error, before-create)_ — Invoice total amount must be greater than zero.
- **PaymentNotExceedsInvoice** _(error, before-create)_ — Payment amount cannot exceed the remaining invoice balance.
