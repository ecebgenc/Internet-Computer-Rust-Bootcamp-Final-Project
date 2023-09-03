// In this final project, you will be building a basic Auction smart contract. In this contract, users will be able to:

// •      	List Items

// •      	Bid for an item

// •      	Update the listing of an item

// •      	Stop the listing of an item

// Listing and Bidding: Users will be able to list an item, similar to creating a proposal in our previous project. 
// After listing the item, other users can bid for it. 
// Bids will be held in a StableBTreeMap, allowing visibility into which principal bid how much, 
// akin to the voted vector in the Proposal project.

// Editing and Stopping Listings: The owner of the item can edit the listing or stop the process at any time.
// When the process is stopped, the highest bidder will become the owner of the item. 
// You can create a specific field called new_owner in the Item struct (feel free to choose any name for the struct and field).

// Item Management: You will maintain a list of items (similar to proposals). 
// Implement the necessary query methods to retrieve a specific item, a list of items, 
// the length of items listed on the contract, the item sold for the most, and the item that has been bid on the most.

// Security Checks: Implement basic security checks to ensure that only the owner of the listing can update or stop it.

use candid::{CandidType, Decode, Deserialize, Encode};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell, u32};
use candid::Principal;


type Memory = VirtualMemory<DefaultMemoryImpl>;


const MAX_VALUE_SIZE: u32 = 5000;


#derive[CandidType]
enum AuctionError {
    UpdateError,
    NoSuchAuction,
    AuctionIsNotActive,
    Expired,
    AccessRejected,
    InvalidChoice,
}


#derive[(CandidType)]
enum BidError {
    BidAmountLessThanCurrent,
    UpdateError,
    NoSuchAuction,
    AuctionIsNotActive,
    Expired,
    ReachMaxBid,
    InvalidChoice,
    OwnerIsNotValid,
}


#[derive(CandidType, Deserialize)]
struct Bid {
    description: String,
    auction: u64, 
    owner: candid::Principal,
    currency: String,
    amount: u32,
    is_active: bool,
}


#[derive(CandidType, Deserialize)]
struct Item {
    title: String,
    description: String,
    owner: candid::Principal,
    new_owner: candid::Principal,
    currency: String,
    amount: u32,
    is_active: bool,
    start_time: String,
    end_time: String,
    bid: Vec<Bid>,
}


#[derive(CandidType, Deserialize)]
struct CreateBid {
    description: String,
    amount: u32,
    currency: String,
    is_active: bool,    
    owner: String,
}


#[derive(CandidType, Deserialize)]
struct CreateItem {
    title: String,
    description: String,
    is_active: bool,
    start_time: String,
    end_time: String,
    currency: String,
    amount: u32,
}


impl Storable for Item {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}


impl BoundedStorable for Item {
    const MAX_SIZE: u32 = MAX_VALUE_SIZE;
    const IS_FIXED_SIZE: bool = false;
}


thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    static PROPOSAL_MAP: RefCell<StableBTreeMap<u64, Auction, Memory>> = RefCell::new(StableBTreeMap::init(
        MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))),
    ));
}

// Get the item
#[ic_cdk::query]
fn get_item(key: u64) -> Option<Item> {
    ITEM_MAP.with(|p| p.borrow().get(&key))
}


// Get the list of all active items in the auction.
#[ic_cdk::query]
fn get_list_of_items() -> Vec<Item> {
    // Create a vector to store the items.
    let mut item_list = Vec::new();

    // Access the ITEM_MAP and iterate through its entries.
    for (_key, item) in ITEM_MAP.with(|p| p.borrow().iter()) {
        // Check if the item is active before adding it to the list.
        if item.is_active {
            item_list.push(item.clone());
        }
    }
    // Return the list of active items.
    item_list
}


// Get number of items
#[ic_cdk::query]
fn get_item_count(key: u64) -> u64 {
    ITEM_MAP.with(|p| p.borrow().len())
}


// Get most bidded item
#[ic_cdk::query]
fn find_most_bidded_item<K, V>(item_map: &StableBTreeMap<K, V>) -> Option<&V>
where
    V: Ord,
{
    // Use the `iter` method to iterate through the items in the map.
    // Find the item with the maximum number of bidders and return it.
    item_map
        .iter()
        .max_by(|(_key_a, item_a), (_key_b, item_b)| item_a.bidders.len().cmp(&item_b.bidders.len()))
        .map(|(_key, item)| item)
}


#[ic_cdk::update]
fn create_item(key: u64, item: CreateItem) -> Option<Item> {
    let value = Item {
        description: item.description, 
        owner: ic_cdk::caller(),
        new_owner: candid::Principal::anonymous(),
        currency: item.currency,
        amount: 0u32,
        is_active: item.is_active,
        start_time: item.start_time,
        end_time: item.end_time,
        bid: vec![],
    };
    ITEM_MAP.with(|p| p.borrow_mut().insert(key, value))
}


#[ic_cdk::update]
fn edit_item(key: u64, item: CreateItem) -> Result<(), AuctionError> {
    ITEM_MAP.with(|p| {
        let old_item_opt = p.borrow().get(&key);
        let old_item = match old_item_opt {
            Some(value) => value,
            None => return Err(AuctionError::NoSuchAuction),
        };

        if ic_cdk::caller() != old_item.owner {
            return Err(AuctionError::AccessRejected);
        }

        if !item.is_active {
            return Err(AuctionError::AuctionIsNotActive);
        }

        let value = Item { 
            description: item.description, 
            owner: ic_cdk::caller(),
            new_owner: candid::Principal::anonymous(),
            currency: item.currency,
            amount: old_item.amount,,
            is_active: item.is_active,,
            start_time: item.start_time,
            end_time: item.end_time,
            bid: old_item.bid, 
        };

        let res = p.borrow_mut().insert(key, value);

        match res {
            Some(_) => Ok(()),
            None => Err(AuctionError::UpdateError),
        }
    })
}


#[ic_cdk::update]
fn end_item(key: u64) -> Result<(), AuctionError> {
    ITEM_MAP.with(|p| {
        let item_opt = p.borrow().get(&key);
        let mut item = match item_opt {
            Some(value) => value,
            None => return Err(AuctionError::NoSuchAuction),
        };

        if ic_cdk::caller() != item.owner {
            return Err(AuctionError::AccessRejected);
        }

        item.is_active = false;

        let mut max_bid_amount = 0;
        let mut max_bid_owner = candid::Principal::anonymous();

        for bid_ in &item.bid {
            if bid_.amount > max_bid_amount {
                max_bid_amount = bid_.amount;
                max_bid_owner = bid_.owner;
            }
        }

        let res = p.borrow_mut().insert(key, item);

        match res {
            Some(_) => Ok(()),
            None => Err(AuctionError::UpdateError),
        }
    })
}


#[ic_cdk::update]
fn bid(key: u64, bid: CreateBid) -> Result<(), BidError> {
    ITEM_MAP.with(|p| {
        //get item from StableBTreeMap
        let item_opt = p.borrow().get(&key);
        let mut item = match item_opt {
            Some(value) => value,
            None => Err(BidError::NoSuchItem),
        };

        let caller: Principal = ic_cdk::caller();

        if item.is_active == false {
            return Err(BidError::AuctionIsNotActive);
        }

        if bid.amount <= item.amount {
            return Err(BidError::BidAmountLessThanCurrent);
        }

        if ic_cdk::caller() == bid.owner {
            return Err(BidError::OwnerIsNotValid);
        }

        item.bid.push(caller);

        let res = p.borrow_mut().insert(key, item);

        match res {
            Some(_) => Ok(()),
            None => Err(BidError::UpdateError),
        }
    })
}