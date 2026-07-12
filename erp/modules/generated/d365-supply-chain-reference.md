# Dynamics 365 Supply Chain Management Reference

_Authored by the **Domain Modeler** hat via `forge`; schema-validated._

**Area:** supply-chain  
**Source:** Microsoft Dynamics 365 Supply Chain Management public documentation (model-reconstructed)

Model-reconstructed digest of the core concepts of Dynamics 365 Supply Chain Management.

## Entities

### Product — Product _(master)_

A distinct item or service that is traded or manufactured.

| Field | Type | Required | Description |
|---|---|---|---|
| `productNumber` | text | yes |  |
| `productName` | text | yes |  |
| `itemGroupId` | lookup | yes |  |
| `unitOfMeasure` | text | yes |  |
| `isStocked` | boolean |  |  |

**Relationships:** `releasedProducts` → ReleasedProduct (one-to-many)

### ReleasedProduct — Released Product _(master)_

A product that has been released to a specific legal entity for use in transactions.

| Field | Type | Required | Description |
|---|---|---|---|
| `productId` | lookup | yes |  |
| `companyId` | lookup | yes |  |
| `inventoryUnit` | text | yes |  |
| `purchasePrice` | money |  |  |
| `salesPrice` | money |  |  |

**Relationships:** `product` → Product (many-to-one)

### PurchaseOrder — Purchase Order _(transactional)_

A document that authorizes the purchase of goods or services from a vendor.

| Field | Type | Required | Description |
|---|---|---|---|
| `orderNumber` | text | yes |  |
| `vendorId` | lookup | yes |  |
| `orderDate` | date | yes |  |
| `status` | picklist | yes |  |
| `totalAmount` | money |  |  |

**Relationships:** `lines` → PurchaseOrderLine (one-to-many)

### PurchaseOrderLine — Purchase Order Line _(transactional)_

A single line item on a purchase order.

| Field | Type | Required | Description |
|---|---|---|---|
| `lineNumber` | integer | yes |  |
| `purchaseOrderId` | lookup | yes |  |
| `productId` | lookup | yes |  |
| `quantity` | decimal | yes |  |
| `unitPrice` | money | yes |  |
| `lineAmount` | money |  |  |

**Relationships:** `purchaseOrder` → PurchaseOrder (many-to-one)

### SalesOrder — Sales Order _(transactional)_

A document that records a customer's request to purchase goods or services.

| Field | Type | Required | Description |
|---|---|---|---|
| `orderNumber` | text | yes |  |
| `customerId` | lookup | yes |  |
| `orderDate` | date | yes |  |
| `status` | picklist | yes |  |
| `totalAmount` | money |  |  |

**Relationships:** `lines` → SalesOrderLine (one-to-many)

### SalesOrderLine — Sales Order Line _(transactional)_

A single line item on a sales order.

| Field | Type | Required | Description |
|---|---|---|---|
| `lineNumber` | integer | yes |  |
| `salesOrderId` | lookup | yes |  |
| `productId` | lookup | yes |  |
| `quantity` | decimal | yes |  |
| `unitPrice` | money | yes |  |
| `lineAmount` | money |  |  |

**Relationships:** `salesOrder` → SalesOrder (many-to-one)

### InventoryTransaction — Inventory Transaction _(transactional)_

A record of a movement of inventory quantity, such as receipt, issue, or transfer.

| Field | Type | Required | Description |
|---|---|---|---|
| `transactionId` | guid | yes |  |
| `productId` | lookup | yes |  |
| `quantity` | decimal | yes |  |
| `direction` | picklist | yes |  |
| `transactionDate` | datetime | yes |  |
| `referenceType` | text |  |  |

**Relationships:** `product` → Product (many-to-one)

### Warehouse — Warehouse _(master)_

A physical location where inventory is stored.

| Field | Type | Required | Description |
|---|---|---|---|
| `warehouseId` | text | yes |  |
| `name` | text | yes |  |
| `siteId` | lookup | yes |  |
| `isActive` | boolean |  |  |

### BillOfMaterials — Bill of Materials _(master)_

A list of components and quantities required to produce a finished product.

| Field | Type | Required | Description |
|---|---|---|---|
| `bomId` | text | yes |  |
| `productId` | lookup | yes |  |
| `name` | text | yes |  |
| `isActive` | boolean |  |  |

**Relationships:** `lines` → BillOfMaterialsLine (one-to-many)

### BillOfMaterialsLine — Bill of Materials Line _(transactional)_

A single component line in a bill of materials.

| Field | Type | Required | Description |
|---|---|---|---|
| `lineNumber` | integer | yes |  |
| `bomId` | lookup | yes |  |
| `componentProductId` | lookup | yes |  |
| `quantity` | decimal | yes |  |
| `unitOfMeasure` | text | yes |  |

**Relationships:** `billOfMaterials` → BillOfMaterials (many-to-one)

### ProductionOrder — Production Order _(transactional)_

An order to manufacture a specific quantity of a product.

| Field | Type | Required | Description |
|---|---|---|---|
| `productionOrderNumber` | text | yes |  |
| `productId` | lookup | yes |  |
| `quantity` | decimal | yes |  |
| `status` | picklist | yes |  |
| `scheduledStartDate` | datetime |  |  |
| `scheduledEndDate` | datetime |  |  |

**Relationships:** `bom` → BillOfMaterials (many-to-one)

### Vendor — Vendor _(master)_

A supplier of goods or services.

| Field | Type | Required | Description |
|---|---|---|---|
| `vendorAccountNumber` | text | yes |  |
| `name` | text | yes |  |
| `vendorGroupId` | lookup | yes |  |
| `isActive` | boolean |  |  |

### Customer — Customer _(master)_

A buyer of goods or services.

| Field | Type | Required | Description |
|---|---|---|---|
| `customerAccountNumber` | text | yes |  |
| `name` | text | yes |  |
| `customerGroupId` | lookup | yes |  |
| `isActive` | boolean |  |  |

## Processes

### ProcureToPay — Procure-to-Pay

End-to-end process from purchase requisition to vendor payment.

1. Create purchase requisition
2. Convert requisition to purchase order
3. Approve purchase order
4. Receive goods against purchase order
5. Record vendor invoice
6. Pay vendor

### OrderToCash — Order-to-Cash

End-to-end process from sales order to customer payment.

1. Create sales order
2. Reserve inventory
3. Pick and pack goods
4. Ship goods
5. Invoice customer
6. Receive payment

### ProductionExecution — Production Execution

Process to manufacture a product from a production order.

1. Create production order
2. Release production order
3. Pick raw materials
4. Start production
5. Report as finished
6. End production order

### InventoryCounting — Inventory Counting

Process to adjust inventory quantities based on physical counts.

1. Create counting journal
2. Enter counted quantities
3. Post journal to adjust inventory

## Rules

- **NegativeInventory** _(error, before-post)_ — Inventory on-hand quantity must not become negative after a transaction.
- **PurchaseOrderApproval** _(error, before-update)_ — Purchase orders above a certain amount must be approved before they can be confirmed.
- **SalesOrderCreditLimit** _(warning, before-create)_ — Sales order cannot exceed the customer's credit limit without approval.
- **BOMConsistency** _(error, before-create)_ — A BOM line cannot reference the same product as the parent BOM product.
