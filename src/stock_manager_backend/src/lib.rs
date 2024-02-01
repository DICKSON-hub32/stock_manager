#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]

struct Item {
    id: u64,
    name: String,
    description: String,
    price: f64,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
struct Stock {
    id: u64,
    item_id: u64,
    quantity: u32,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
struct Transaction {
    id: u64,
    stock_id: u64,
    transaction_type: TransactionType,
    quantity: u32,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize)]
enum TransactionType {
    Inflow,
    Outflow,
}

// implementing the Storable and BoundedStorable traits for our structs

impl Storable for Item {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Item {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

impl Storable for Stock {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Stock {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

impl Storable for Transaction {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Transaction {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

impl Storable for TransactionType {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for TransactionType {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

//  setting up our thread-local variables that will hold our canister's state
thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );
    static ITEM_ID: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );
    static STOCK_ID: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1))), 0)
            .expect("Cannot create a counter")
    );
    static TRANSACTION_ID: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2))), 0)
            .expect("Cannot create a counter")
    );
    static ITEM_STR: RefCell<StableBTreeMap<u64, Item, Memory>> = RefCell::new(
        StableBTreeMap::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(3))))
    );
    static STOCK_STR: RefCell<StableBTreeMap<u64, Stock, Memory>> = RefCell::new(
        StableBTreeMap::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(4))))
    );
    static TRANSACTION_STR: RefCell<StableBTreeMap<u64, Transaction, Memory>> = RefCell::new(
        StableBTreeMap::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(5))))
    );
}

// Payload structures for adding or updating items, stock, and transactions
#[derive(candid::CandidType, Serialize, Deserialize)]
struct ItemPayload {
    name: String,
    description: String,
    price: f64,
}

#[derive(candid::CandidType, Serialize, Deserialize)]
struct StockPayload {
    item_id: u64,
    quantity: u32,
}

#[derive(candid::CandidType, Serialize, Deserialize)]
struct TransactionPayload {
    stock_id: u64,
    transaction_type: TransactionType,
    quantity: u32,
}

// Candid queries for retrieving data
#[ic_cdk::query]
fn get_items() -> Result<Vec<Item>, Error> {
    let items = ITEM_STR.with(|m| {
        m.borrow()
            .iter()
            .map(|(_, v)| v.clone())
            .collect::<Vec<_>>()
    });
    if items.len() == 0 {
        return Err(Error::NotFound {
            msg: "No Items found".to_string(),
        });
    }
    Ok(items)
}

#[ic_cdk::query]
fn get_item_by_id(id: u64) -> Result<Item, Error> {
    ITEM_STR.with(|service| {
        service.borrow_mut().get(&id).ok_or(Error::NotFound {
            msg: format!("Item with the id={} not found", id),
        })
    })
}

// Update functions for modifying data
#[ic_cdk::update]
fn add_item(payload: ItemPayload) -> Result<Item, Error> {
    let id = ITEM_ID
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("cannot increment id counter");

    let item = Item {
        id,
        name: payload.name,
        description: payload.description,
        price: payload.price,
    };

    ITEM_STR.with(|m| m.borrow_mut().insert(id, item.clone()));
    Ok(item)
}

#[ic_cdk::update]
fn update_item(id: u64, payload: ItemPayload) -> Result<Item, Error> {
    ITEM_STR.with(|m| {
        let mut item = m.borrow_mut().get(&id).ok_or(Error::NotFound {
            msg: format!("Item with the id={} not found", id),
        })?;

        item.name = payload.name;
        item.description = payload.description;
        item.price = payload.price;

        m.borrow_mut().insert(id, item.clone());
        Ok(item)
    })
}

#[ic_cdk::update]
fn delete_item(id: u64) -> Result<(), Error> {
    ITEM_STR.with(|m| {
        if m.borrow_mut().remove(&id).is_none() {
            Err(Error::NotFound {
                msg: format!("Item with the id={} not found", id),
            })
        } else {
            Ok(())
        }
    })
}

// Candid queries for retrieving data
#[ic_cdk::query]
fn get_stock() -> Result<Vec<Stock>, Error> {
    let stock = STOCK_STR.with(|m| {
        m.borrow()
            .iter()
            .map(|(_, v)| v.clone())
            .collect::<Vec<_>>()
    });
    if stock.len() == 0 {
        return Err(Error::NotFound {
            msg: "No Stock found".to_string(),
        });
    }
    Ok(stock)
}

#[ic_cdk::query]
fn get_stock_by_id(id: u64) -> Result<Stock, Error> {
    STOCK_STR.with(|service| {
        service.borrow_mut().get(&id).ok_or(Error::NotFound {
            msg: format!("Stock with the id={} not found", id),
        })
    })
}

// Update functions for modifying data

#[ic_cdk::update]
fn add_stock(payload: StockPayload) -> Result<Stock, Error> {
    let id = STOCK_ID
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("cannot increment id counter");

    let stock = Stock {
        id,
        item_id: payload.item_id,
        quantity: payload.quantity,
    };

    STOCK_STR.with(|m| m.borrow_mut().insert(id, stock.clone()));
    Ok(stock)
}

#[ic_cdk::update]
fn update_stock(id: u64, payload: StockPayload) -> Result<Stock, Error> {
    STOCK_STR.with(|m| {
        let mut stock = m.borrow_mut().get(&id).ok_or(Error::NotFound {
            msg: format!("Stock with the id={} not found", id),
        })?;

        stock.item_id = payload.item_id;
        stock.quantity = payload.quantity;

        m.borrow_mut().insert(id, stock.clone());
        Ok(stock)
    })
}

#[ic_cdk::update]
fn delete_stock(id: u64) -> Result<(), Error> {
    STOCK_STR.with(|m| {
        if m.borrow_mut().remove(&id).is_none() {
            Err(Error::NotFound {
                msg: format!("Stock with the id={} not found", id),
            })
        } else {
            Ok(())
        }
    })
}

// Candid queries for retrieving data
#[ic_cdk::query]
fn get_transactions() -> Result<Vec<Transaction>, Error> {
    let transactions = TRANSACTION_STR.with(|m| {
        m.borrow()
            .iter()
            .map(|(_, v)| v.clone())
            .collect::<Vec<_>>()
    });
    if transactions.len() == 0 {
        return Err(Error::NotFound {
            msg: "No Transactions found".to_string(),
        });
    }
    Ok(transactions)
}

// Update functions for modifying data
#[ic_cdk::update]
fn add_transaction(payload: TransactionPayload) -> Result<Transaction, Error> {
    let id = TRANSACTION_ID
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("cannot increment id counter");

    let transaction = Transaction {
        id,
        stock_id: payload.stock_id,
        transaction_type: payload.transaction_type,
        quantity: payload.quantity,
    };

    TRANSACTION_STR.with(|m| m.borrow_mut().insert(id, transaction.clone()));
    Ok(transaction)
}

#[ic_cdk::update]
fn update_transaction(id: u64, payload: TransactionPayload) -> Result<Transaction, Error> {
    TRANSACTION_STR.with(|m| {
        let mut transaction = m.borrow_mut().get(&id).ok_or(Error::NotFound {
            msg: format!("Transaction with the id={} not found", id),
        })?;

        transaction.stock_id = payload.stock_id;
        transaction.transaction_type = payload.transaction_type;
        transaction.quantity = payload.quantity;

        m.borrow_mut().insert(id, transaction.clone());
        Ok(transaction)
    })
}

#[ic_cdk::update]
fn delete_transaction(id: u64) -> Result<(), Error> {
    TRANSACTION_STR.with(|m| {
        if m.borrow_mut().remove(&id).is_none() {
            Err(Error::NotFound {
                msg: format!("Transaction with the id={} not found", id),
            })
        } else {
            Ok(())
        }
    })
}

// Define errors that can occur in the application

#[derive(candid::CandidType, Debug, Serialize, Deserialize)]
enum Error {
    NotFound { msg: String },
}

// need this to generate candid
ic_cdk::export_candid!();
