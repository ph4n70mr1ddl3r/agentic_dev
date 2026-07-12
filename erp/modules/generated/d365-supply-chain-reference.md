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

**Relationships:** `inventoryOnHand` → InventoryOnHand (one-to-many); `billOfMaterials` → BillOfMaterials (one-to-many)

### PurchaseOrder — Purchase Order _(transactional)_

A document that authorizes the purchase of products from a vendor.

| Field | Type | Required | Description |
|---|---|---|---|
| `orderNumber` | text | yes |  |
| `vendorId` | lookup | yes |  |
| `orderDate` | date | yes |  |
| `status` | picklist | yes |  |

**Relationships:** `lines` → PurchaseOrderLine (one-to-many)

### PurchaseOrderLine — Purchase Order Line _(transactional)_

A line item on a purchase order specifying product, quantity, and price.

| Field | Type | Required | Description |
|---|---|---|---|
| `lineNumber` | integer | yes |  |
| `productId` | lookup | yes |  |
| `quantity` | decimal | yes |  |
| `unitPrice` | money | yes |  |

**Relationships:** `purchaseOrder` → PurchaseOrder (many-to-one)

### SalesOrder — Sales Order _(transactional)_

A document that records a customer's request to purchase products.

| Field | Type | Required | Description |
|---|---|---|---|
| `orderNumber` | text | yes |  |
| `customerId` | lookup | yes |  |
| `orderDate` | date | yes |  |
| `status` | picklist | yes |  |

**Relationships:** `lines` → SalesOrderLine (one-to-many)

### SalesOrderLine — Sales Order Line _(transactional)_

A line item on a sales order specifying product, quantity, and price.

| Field | Type | Required | Description |
|---|---|---|---|
| `lineNumber` | integer | yes |  |
| `productId` | lookup | yes |  |
| `quantity` | decimal | yes |  |
| `unitPrice` | money | yes |  |

**Relationships:** `salesOrder` → SalesOrder (many-to-one)

### InventoryOnHand — Inventory On-Hand _(transactional)_

Current inventory quantity of a product at a specific warehouse and location.

| Field | Type | Required | Description |
|---|---|---|---|
| `productId` | lookup | yes |  |
| `warehouseId` | lookup | yes |  |
| `quantity` | decimal | yes |  |
| `lastUpdated` | datetime | yes |  |

**Relationships:** `product` → Product (many-to-one)

### Warehouse — Warehouse _(master)_

A physical location where inventory is stored.

| Field | Type | Required | Description |
|---|---|---|---|
| `warehouseId` | text | yes |  |
| `warehouseName` | text | yes |  |
| `siteId` | lookup | yes |  |

### BillOfMaterials — Bill of Materials _(master)_

A list of components and quantities required to manufacture a product.

| Field | Type | Required | Description |
|---|---|---|---|
| `bomId` | text | yes |  |
| `productId` | lookup | yes |  |
| `quantity` | decimal | yes |  |
| `validFrom` | date | yes |  |

**Relationships:** `lines` → BillOfMaterialsLine (one-to-many)

### BillOfMaterialsLine — Bill of Materials Line _(master)_

A component line in a bill of materials.

| Field | Type | Required | Description |
|---|---|---|---|
| `lineNumber` | integer | yes |  |
| `componentProductId` | lookup | yes |  |
| `quantity` | decimal | yes |  |

**Relationships:** `billOfMaterials` → BillOfMaterials (many-to-one)

### ProductionOrder — Production Order _(transactional)_

An order to manufacture a specific quantity of a product.

| Field | Type | Required | Description |
|---|---|---|---|
| `productionOrderNumber` | text | yes |  |
| `productId` | lookup | yes |  |
| `quantity` | decimal | yes |  |
| `status` | picklist | yes |  |

**Relationships:** `bom` → BillOfMaterials (many-to-one)

### Vendor — Vendor _(master)_

A supplier of products or services.

| Field | Type | Required | Description |
|---|---|---|---|
| `vendorId` | text | yes |  |
| `vendorName` | text | yes |  |
| `vendorGroup` | lookup | yes |  |

### Customer — Customer _(master)_

A buyer of products or services.

| Field | Type | Required | Description |
|---|---|---|---|
| `customerId` | text | yes |  |
| `customerName` | text | yes |  |
| `customerGroup` | lookup | yes |  |

## Processes

### ProcureToPay — Procure-to-Pay

End-to-end process from purchase requisition to vendor payment.

1. Create purchase requisition
2. Convert to purchase order
3. Approve purchase order
4. Receive products against purchase order
5. Record vendor invoice
6. Pay vendor

### OrderToCash — Order-to-Cash

End-to-end process from sales order to customer payment.

1. Create sales order
2. Reserve inventory
3. Pick and pack products
4. Ship products
5. Invoice customer
6. Receive payment

### ProductionExecution — Production Execution

Process to manufacture products from raw materials.

1. Create production order
2. Release production order
3. Pick raw materials
4. Start production
5. Report as finished
6. End production order

## Rules

- **NegativeInventoryNotAllowed** _(error, before-update)_ — Inventory on-hand quantity cannot go below zero.
- **PurchaseOrderApprovalRequired** _(error, on-submit)_ — Purchase orders above a certain value must be approved before confirmation.
- **SalesOrderCreditLimitCheck** _(warning, before-create)_ — Sales order cannot exceed customer's credit limit.
- **BOMVersionValidity** _(error, before-create)_ — A BOM version must have a valid-from date and cannot overlap with another active version for the same product.
