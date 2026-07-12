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

### ChartOfAccounts — Chart of Accounts _(master)_

The structured list of all general ledger accounts used by a company.

| Field | Type | Required | Description |
|---|---|---|---|
| `accountId` | text | yes |  |
| `name` | text | yes |  |
| `type` | picklist | yes |  |

### LedgerAccount — Ledger Account _(master)_

An individual account in the chart of accounts, used for posting transactions.

| Field | Type | Required | Description |
|---|---|---|---|
| `mainAccountId` | text | yes |  |
| `name` | text | yes |  |
| `balanceControl` | boolean |  |  |

**Relationships:** `chartOfAccounts` → ChartOfAccounts (many-to-one)

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
| `currency` | text |  |  |

**Relationships:** `journalEntry` → JournalEntry (many-to-one); `ledgerAccount` → LedgerAccount (many-to-one)

### GeneralLedger — General Ledger _(transactional)_

The central repository of all financial transactions posted to the ledger.

| Field | Type | Required | Description |
|---|---|---|---|
| `transactionId` | guid | yes |  |
| `postingDate` | date | yes |  |
| `amount` | money | yes |  |
| `accountId` | lookup | yes |  |

**Relationships:** `ledgerAccount` → LedgerAccount (many-to-one)

### FiscalPeriod — Fiscal Period _(reference)_

A time period used for financial reporting and closing.

| Field | Type | Required | Description |
|---|---|---|---|
| `periodCode` | text | yes |  |
| `startDate` | date | yes |  |
| `endDate` | date | yes |  |
| `status` | picklist |  |  |

### Currency — Currency _(reference)_

A monetary unit used for transactions and reporting.

| Field | Type | Required | Description |
|---|---|---|---|
| `currencyCode` | text | yes |  |
| `name` | text | yes |  |
| `exchangeRate` | decimal |  |  |

### Vendor — Vendor _(master)_

A supplier from whom goods or services are purchased.

| Field | Type | Required | Description |
|---|---|---|---|
| `vendorAccount` | text | yes |  |
| `name` | text | yes |  |
| `currencyId` | lookup |  |  |

### Customer — Customer _(master)_

A buyer of goods or services.

| Field | Type | Required | Description |
|---|---|---|---|
| `customerAccount` | text | yes |  |
| `name` | text | yes |  |
| `currencyId` | lookup |  |  |

### PurchaseInvoice — Purchase Invoice _(transactional)_

An invoice received from a vendor for goods or services.

| Field | Type | Required | Description |
|---|---|---|---|
| `invoiceNumber` | text | yes |  |
| `vendorId` | lookup | yes |  |
| `invoiceDate` | date | yes |  |
| `totalAmount` | money | yes |  |

**Relationships:** `vendor` → Vendor (many-to-one)

### SalesInvoice — Sales Invoice _(transactional)_

An invoice issued to a customer for goods or services.

| Field | Type | Required | Description |
|---|---|---|---|
| `invoiceNumber` | text | yes |  |
| `customerId` | lookup | yes |  |
| `invoiceDate` | date | yes |  |
| `totalAmount` | money | yes |  |

**Relationships:** `customer` → Customer (many-to-one)

### Payment — Payment _(transactional)_

A payment made to a vendor or received from a customer.

| Field | Type | Required | Description |
|---|---|---|---|
| `paymentId` | guid | yes |  |
| `paymentDate` | date | yes |  |
| `amount` | money | yes |  |
| `direction` | picklist | yes |  |

### Budget — Budget _(master)_

A financial plan for a specific period, used for control and reporting.

| Field | Type | Required | Description |
|---|---|---|---|
| `budgetId` | text | yes |  |
| `fiscalYear` | integer | yes |  |
| `status` | picklist |  |  |

## Processes

### JournalPosting — Journal Posting

Validate and post a journal entry to the general ledger.

1. Create or open a journal entry
2. Enter journal lines with debit and credit amounts
3. Validate the entry balances (debits = credits)
4. Post the journal entry
5. Update general ledger account balances

### VendorInvoiceProcessing — Vendor Invoice Processing

Record and pay a purchase invoice from a vendor.

1. Receive purchase invoice from vendor
2. Match invoice to purchase order or receipt
3. Record invoice in accounts payable
4. Approve invoice for payment
5. Generate payment to vendor
6. Post payment and settle invoice

### CustomerInvoiceProcessing — Customer Invoice Processing

Issue and collect payment for a sales invoice.

1. Create sales invoice for customer
2. Post invoice to accounts receivable
3. Send invoice to customer
4. Receive payment from customer
5. Apply payment to invoice
6. Post payment and settle invoice

### PeriodClosing — Period Closing

Close a fiscal period and prepare financial statements.

1. Review and post all pending transactions
2. Perform adjustments (accruals, deferrals)
3. Revalue foreign currency balances
4. Run allocation rules
5. Generate financial reports (trial balance, P&L, balance sheet)
6. Close the period

### BudgetPlanning — Budget Planning

Create and approve a budget for a fiscal year.

1. Define budget plan structure
2. Enter budget amounts by account and period
3. Review and adjust budget
4. Approve budget
5. Load budget into general ledger for control

## Rules

- **BalancedEntry** _(error, before-post)_ — A journal entry's debits must equal its credits before posting.
- **AccountTypeValidation** _(warning, before-post)_ — Debits and credits must be posted to appropriate account types (e.g., asset accounts normally have debit balances).
- **PeriodStatusCheck** _(error, before-post)_ — Transactions can only be posted to open fiscal periods.
- **CurrencyConsistency** _(error, before-post)_ — All lines in a journal entry must use the same currency unless multicurrency is enabled.
- **MandatoryFields** _(error, before-post)_ — Required fields (e.g., account, date, amount) must be filled before posting.
- **BudgetControl** _(error, before-post)_ — Expenditures must not exceed available budget when budget control is active.
- **InvoiceMatching** _(warning, before-post)_ — Purchase invoice amounts must match purchase order or receipt within tolerance.
