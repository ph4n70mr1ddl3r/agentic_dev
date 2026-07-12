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

**Relationships:** `inventoryTransactions` → InventoryTransaction (one-to-many); `purchaseOrderLines` → PurchaseOrderLine (one-to-many); `salesOrderLines` → SalesOrderLine (one-to-many)

### PurchaseOrder — Purchase Order _(transactional)_

A document that authorizes the purchase of products from a vendor.

| Field | Type | Required | Description |
|---|---|---|---|
| `purchaseOrderNumber` | text | yes |  |
| `vendorAccountNumber` | lookup | yes |  |
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
| `receivedQuantity` | decimal |  |  |

**Relationships:** `purchaseOrder` → PurchaseOrder (many-to-one); `product` → Product (many-to-one)

### SalesOrder — Sales Order _(transactional)_

A document that records a customer's request to purchase products.

| Field | Type | Required | Description |
|---|---|---|---|
| `salesOrderNumber` | text | yes |  |
| `customerAccountNumber` | lookup | yes |  |
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
| `shippedQuantity` | decimal |  |  |

**Relationships:** `salesOrder` → SalesOrder (many-to-one); `product` → Product (many-to-one)

### InventoryTransaction — Inventory Transaction _(transactional)_

Records a movement of inventory (receipt, issue, transfer).

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
| `warehouseName` | text | yes |  |
| `siteId` | lookup | yes |  |

**Relationships:** `inventoryTransactions` → InventoryTransaction (one-to-many)

### Vendor — Vendor _(master)_

A supplier of products or services.

| Field | Type | Required | Description |
|---|---|---|---|
| `vendorAccountNumber` | text | yes |  |
| `vendorName` | text | yes |  |
| `currencyCode` | text | yes |  |

**Relationships:** `purchaseOrders` → PurchaseOrder (one-to-many)

### Customer — Customer _(master)_

A buyer of products or services.

| Field | Type | Required | Description |
|---|---|---|---|
| `customerAccountNumber` | text | yes |  |
| `customerName` | text | yes |  |
| `currencyCode` | text | yes |  |

**Relationships:** `salesOrders` → SalesOrder (one-to-many)

## Processes

### PurchaseOrderProcurement — Purchase Order Procurement

Process of creating, approving, and receiving a purchase order.

1. Create purchase order header with vendor and dates
2. Add purchase order lines with products and quantities
3. Submit for approval
4. Approve purchase order
5. Receive products against purchase order lines
6. Update inventory transactions with receipt

### SalesOrderFulfillment — Sales Order Fulfillment

Process of creating, confirming, and shipping a sales order.

1. Create sales order header with customer and dates
2. Add sales order lines with products and quantities
3. Confirm sales order
4. Reserve inventory
5. Pick and pack products
6. Ship products and update inventory transactions

### InventoryTransfer — Inventory Transfer

Process of moving inventory from one warehouse to another.

1. Create transfer order
2. Specify source and destination warehouses
3. Add products and quantities
4. Ship from source warehouse
5. Receive at destination warehouse
6. Update inventory transactions for both warehouses

## Rules

- **NegativeInventoryNotAllowed** _(error, before-update)_ — Inventory quantity for a product cannot go below zero unless configured otherwise.
- **PurchaseOrderLineQuantityPositive** _(error, before-create)_ — Quantity on a purchase order line must be greater than zero.
- **SalesOrderLineQuantityPositive** _(error, before-create)_ — Quantity on a sales order line must be greater than zero.
- **ReceivedQuantityCannotExceedOrderedQuantity** _(error, before-update)_ — The received quantity on a purchase order line cannot exceed the ordered quantity.
- **ShippedQuantityCannotExceedOrderedQuantity** _(error, before-update)_ — The shipped quantity on a sales order line cannot exceed the ordered quantity.
