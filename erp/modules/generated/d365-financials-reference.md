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
| `mainAccount` | text | yes |  |
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
| `lineNo` | integer | yes |  |
| `account` | lookup | yes |  |
| `debitAmount` | money |  |  |
| `creditAmount` | money |  |  |
| `currency` | text |  |  |

**Relationships:** `journalEntry` → JournalEntry (many-to-one); `ledgerAccount` → LedgerAccount (many-to-one)

### Customer — Customer _(master)_

A person or organization that purchases goods or services on credit.

| Field | Type | Required | Description |
|---|---|---|---|
| `customerId` | text | yes |  |
| `name` | text | yes |  |
| `creditLimit` | money |  |  |

### Vendor — Vendor _(master)_

A supplier from whom goods or services are purchased.

| Field | Type | Required | Description |
|---|---|---|---|
| `vendorId` | text | yes |  |
| `name` | text | yes |  |
| `paymentTerms` | text |  |  |

### Invoice — Invoice _(transactional)_

A document issued by a seller to a buyer for goods or services provided.

| Field | Type | Required | Description |
|---|---|---|---|
| `invoiceId` | text | yes |  |
| `invoiceDate` | date | yes |  |
| `dueDate` | date | yes |  |
| `totalAmount` | money | yes |  |
| `status` | picklist | yes |  |

**Relationships:** `customer` → Customer (many-to-one); `vendor` → Vendor (many-to-one)

### Payment — Payment _(transactional)_

A payment made to or received from a customer or vendor.

| Field | Type | Required | Description |
|---|---|---|---|
| `paymentId` | text | yes |  |
| `paymentDate` | date | yes |  |
| `amount` | money | yes |  |
| `direction` | picklist | yes |  |

**Relationships:** `customer` → Customer (many-to-one); `vendor` → Vendor (many-to-one)

### GeneralLedger — General Ledger _(reference)_

The central repository of all financial transactions posted from subledgers.

| Field | Type | Required | Description |
|---|---|---|---|
| `transactionId` | guid | yes |  |
| `postingDate` | date | yes |  |
| `amount` | money | yes |  |

## Processes

### JournalPosting — Journal Posting

Validate and post a journal entry to the general ledger.

1. Create journal entry header
2. Add journal lines with debits and credits
3. Validate that debits equal credits
4. Post the journal entry
5. Update ledger account balances

### InvoiceProcessing — Invoice Processing

Create and post an invoice for a customer or from a vendor.

1. Create invoice header
2. Add invoice lines
3. Validate invoice totals
4. Post the invoice
5. Update customer/vendor balance

### PaymentSettlement — Payment Settlement

Apply a payment to outstanding invoices.

1. Receive or make payment
2. Identify open invoices
3. Apply payment to invoices
4. Post settlement
5. Update customer/vendor balances

## Rules

- **BalancedEntry** _(error, before-post)_ — A journal entry's debits must equal its credits before posting.
- **CreditLimitCheck** _(warning, before-post)_ — An invoice cannot exceed the customer's credit limit.
- **RequiredFields** _(error, before-create)_ — Mandatory fields must be filled before saving.
