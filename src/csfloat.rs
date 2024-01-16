use std::collections::HashSet;

use crate::types::ListingId;

pub struct CsfloatScheduler {
    // for fast existance check
    hs: HashSet<ListingId>,
    // contains ListingId in ordered way
    v: Vec<ListingId>,
    // pointer to a item
    idx: usize,
    // mb also add Vec for temporary failed listings
}

impl CsfloatScheduler {
    pub fn new() -> Self {
        CsfloatScheduler {
            hs: HashSet::new(),
            v: Vec::<ListingId>::new(),
            idx: 0,
        }
    }

    pub fn get_size(&self) -> usize {
        self.v.len()
    }

    pub fn upsert_listing(&mut self, listing_id: &ListingId) {
        if !self.hs.contains(listing_id) {
            self.hs.insert(listing_id.clone());
            self.v.push(listing_id.clone());
        }
    }

    pub fn remove_listing(&mut self, listing_id: &ListingId) {
        if self.hs.contains(listing_id) {
            self.hs.remove(listing_id);
            self.v.retain(|x| *x != *listing_id);
        }
    }

    pub fn get_next(&mut self) -> Option<ListingId> {
        if self.idx == 0 && self.v.is_empty() {
            return None;
        }

        if self.idx >= self.v.len() {
            self.idx = 0;
        }
        let result = self.v.get(self.idx).unwrap();
        self.idx += 1;

        Some(result.to_string())
    }
}
