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
| `isPurchased` | boolean |  |  |
| `isManufactured` | boolean |  |  |

**Relationships:** `inventDim` → InventoryDimension (one-to-many); `bom` → BillOfMaterials (one-to-many)

### PurchaseOrder — Purchase Order _(transactional)_

A document that authorizes a purchase transaction with a vendor.

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
| `productId` | lookup | yes |  |
| `quantity` | decimal | yes |  |
| `unitPrice` | money | yes |  |
| `lineAmount` | money |  |  |

**Relationships:** `purchaseOrder` → PurchaseOrder (many-to-one)

### SalesOrder — Sales Order _(transactional)_

A document that records a customer's request to purchase products or services.

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
| `productId` | lookup | yes |  |
| `quantity` | decimal | yes |  |
| `unitPrice` | money | yes |  |
| `lineAmount` | money |  |  |

**Relationships:** `salesOrder` → SalesOrder (many-to-one)

### InventoryDimension — Inventory Dimension _(reference)_

A dimension that tracks inventory by site, warehouse, location, batch, serial, etc.

| Field | Type | Required | Description |
|---|---|---|---|
| `dimensionId` | text | yes |  |
| `dimensionType` | picklist | yes |  |
| `value` | text | yes |  |

### Warehouse — Warehouse _(master)_

A physical location where inventory is stored.

| Field | Type | Required | Description |
|---|---|---|---|
| `warehouseId` | text | yes |  |
| `name` | text | yes |  |
| `siteId` | lookup | yes |  |

### BillOfMaterials — Bill of Materials _(master)_

A list of components and quantities required to manufacture a product.

| Field | Type | Required | Description |
|---|---|---|---|
| `bomId` | text | yes |  |
| `productId` | lookup | yes |  |
| `name` | text |  |  |
| `isActive` | boolean |  |  |

**Relationships:** `lines` → BillOfMaterialsLine (one-to-many)

### BillOfMaterialsLine — Bill of Materials Line _(transactional)_

A single component line in a bill of materials.

| Field | Type | Required | Description |
|---|---|---|---|
| `lineNumber` | integer | yes |  |
| `componentProductId` | lookup | yes |  |
| `quantity` | decimal | yes |  |
| `unitOfMeasure` | text | yes |  |

**Relationships:** `bom` → BillOfMaterials (many-to-one)

### ProductionOrder — Production Order _(transactional)_

An order to manufacture a specific quantity of a product.

| Field | Type | Required | Description |
|---|---|---|---|
| `productionOrderNumber` | text | yes |  |
| `productId` | lookup | yes |  |
| `quantity` | decimal | yes |  |
| `status` | picklist | yes |  |
| `scheduledStartDate` | date |  |  |
| `scheduledEndDate` | date |  |  |

**Relationships:** `bom` → BillOfMaterials (many-to-one); `route` → Route (many-to-one)

### Route — Route _(master)_

A sequence of operations required to manufacture a product.

| Field | Type | Required | Description |
|---|---|---|---|
| `routeId` | text | yes |  |
| `productId` | lookup | yes |  |
| `name` | text |  |  |

**Relationships:** `operations` → RouteOperation (one-to-many)

### RouteOperation — Route Operation _(transactional)_

A single operation in a production route.

| Field | Type | Required | Description |
|---|---|---|---|
| `operationNumber` | integer | yes |  |
| `operationId` | lookup | yes |  |
| `runTime` | decimal |  |  |
| `setupTime` | decimal |  |  |

**Relationships:** `route` → Route (many-to-one)

### TransferOrder — Transfer Order _(transactional)_

An order to move inventory from one warehouse to another.

| Field | Type | Required | Description |
|---|---|---|---|
| `transferOrderNumber` | text | yes |  |
| `fromWarehouseId` | lookup | yes |  |
| `toWarehouseId` | lookup | yes |  |
| `status` | picklist | yes |  |
| `transferDate` | date |  |  |

**Relationships:** `lines` → TransferOrderLine (one-to-many)

### TransferOrderLine — Transfer Order Line _(transactional)_

A single line item on a transfer order.

| Field | Type | Required | Description |
|---|---|---|---|
| `lineNumber` | integer | yes |  |
| `productId` | lookup | yes |  |
| `quantity` | decimal | yes |  |

**Relationships:** `transferOrder` → TransferOrder (many-to-one)

## Processes

### ProcureToPay — Procure to Pay

End-to-end process from purchase requisition to vendor payment.

1. Create purchase requisition
2. Convert requisition to purchase order
3. Approve purchase order
4. Receive products against purchase order
5. Record vendor invoice
6. Pay vendor

### OrderToCash — Order to Cash

End-to-end process from sales order to customer payment.

1. Create sales order
2. Reserve inventory
3. Pick and pack products
4. Ship products
5. Invoice customer
6. Receive payment

### ProductionExecution — Production Execution

Process to manufacture a product from a production order.

1. Create production order
2. Estimate production order
3. Schedule production order
4. Release production order
5. Start production order
6. Report as finished
7. End production order

### InventoryTransfer — Inventory Transfer

Process to move inventory between warehouses.

1. Create transfer order
2. Pick inventory from source warehouse
3. Ship inventory
4. Receive inventory at destination warehouse

## Rules

- **PurchaseOrderApproval** _(error, before-update)_ — Purchase orders above a certain amount must be approved before they can be confirmed.
- **SalesOrderCreditLimit** _(error, before-create)_ — Sales orders cannot exceed the customer's credit limit without approval.
- **NegativeInventory** _(error, before-post)_ — Inventory on-hand quantity cannot go negative for physical transactions.
- **BOMConsistency** _(error, before-create)_ — A BOM must have at least one line and cannot contain itself as a component.
- **ProductionOrderScheduling** _(warning, before-update)_ — A production order must have a scheduled start and end date before release.
