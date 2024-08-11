use std::{cell::RefCell, rc::Rc};

use crate::value::{StringObject, Value};

type EntryKey = Rc<RefCell<StringObject>>;

#[derive(Clone)]
enum TableEntry {
    Value(Entry),
    Tombstone,
    Empty,
}

impl TableEntry {
    pub fn get_value(&mut self) -> TableEntry {
        // TODO: check how take in Option is implemented and try to do something similar
        match self {
            TableEntry::Value(v) => {
                let result = TableEntry::Value(v.clone());
                *self = TableEntry::Empty;
                result
            }
            TableEntry::Tombstone => TableEntry::Tombstone,
            TableEntry::Empty => TableEntry::Empty,
        }
    }
}

#[derive(Clone)]
struct Entry {
    key: EntryKey,
    value: Value,
}

impl Entry {
    fn new(key: EntryKey, value: Value) -> Self {
        Self { key, value }
    }
}

pub enum InsertResult {
    Added,
    Replaced,
}

pub struct KeyNotFound {}

pub struct Table {
    /// Number of entires and tombstones in the table
    entries_count: usize,
    entries: Vec<TableEntry>,
}

impl Table {
    pub fn new() -> Self {
        Table {
            entries_count: 0,
            entries: Vec::new(),
        }
    }

    const TABLE_MAX_LOAD: f32 = 0.75;

    pub fn insert(&mut self, key: EntryKey, value: Value) -> InsertResult {
        if self.entries_count + 1 > (self.entries.len() as f32 * Self::TABLE_MAX_LOAD) as usize {
            let new_capacity = self.entries.len() * 2;
            self.adjust_size(new_capacity);
        }
        let entry_index = Self::find_entry(&self.entries, &key);
        let entry = &self.entries[entry_index];
        let result = match entry {
            TableEntry::Value(_) => InsertResult::Replaced,
            TableEntry::Empty => {
                self.entries_count += 1;
                return InsertResult::Added;
            }
            TableEntry::Tombstone => InsertResult::Replaced,
        };
        let new_entry = Entry::new(key, value);
        self.entries[entry_index] = TableEntry::Value(new_entry);
        result
    }

    pub fn insert_all_from(from: &mut Table, to: &mut Table) {
        for entry in &from.entries {
            if let TableEntry::Value(entry) = entry {
                to.insert(entry.key.clone(), entry.value.clone());
            }
        }
    }

    pub fn get(&self, key: &EntryKey) -> Result<&Value, KeyNotFound> {
        if self.entries_count == 0 {
            return Err(KeyNotFound {});
        }

        let result_id = Self::find_entry(&self.entries, key);
        let entry = &self.entries[result_id];
        match entry {
            TableEntry::Value(v) => Ok(&v.value),
            _ => Err(KeyNotFound {}),
        }
    }

    pub fn remove(&mut self, key: &EntryKey) -> Result<(), KeyNotFound> {
        if self.entries_count == 0 {
            return Err(KeyNotFound {});
        }
        let entry_id = Self::find_entry(&self.entries, key);
        match &self.entries[entry_id] {
            TableEntry::Value(_) => {
                self.entries[entry_id] = TableEntry::Tombstone;
                Ok(())
            }
            TableEntry::Tombstone => Err(KeyNotFound {}),
            TableEntry::Empty => Err(KeyNotFound {}),
        }
    }

    // We use entries instead of passing self so that we can use it on `adjust_size` for new entries array
    fn find_entry(entries: &[TableEntry], key: &EntryKey) -> usize {
        let mut index = key.borrow().get_hash() as usize % entries.len();
        let mut first_tombstone_index: Option<usize> = None;
        // Thanks to the load factor and the way we grow the array there will never be a case of infinite loop
        loop {
            let entry = &entries[index];
            match entry {
                TableEntry::Value(entry) => {
                    // Thanks to this we don't have to just if string (possibly very long) are equal - we just
                    // check if underlying pointers point to the same place in memory
                    // FIXME: CURRENLTY ITS NOT WORKING
                    // TODO: in order for this to work, in the compiler everywhere where StringObject::new is called
                    // it must use String Interning
                    // We can probalby just change the implementation of "new" itself (with may be changing name)
                    // and leave the rest of code as it is
                    if Rc::ptr_eq(&entry.key, key) {
                        return index;
                    }
                }
                TableEntry::Tombstone => match first_tombstone_index {
                    Some(_) => {}
                    None => first_tombstone_index = Some(index),
                },
                TableEntry::Empty => match first_tombstone_index {
                    Some(id) => return id,
                    None => return index,
                },
            }

            index = (index + 1) % entries.len();
        }
    }

    fn adjust_size(&mut self, new_capacity: usize) {
        let mut new_entries: Vec<TableEntry> = vec![TableEntry::Empty; new_capacity];
        let mut new_count: usize = 0;

        for i in 0..self.entries.len() {
            if let TableEntry::Value(entry) = &self.entries[i] {
                // We know that here we are not adding new elemnts, so the count never changes here
                let dest_index = Self::find_entry(&new_entries, &entry.key);
                new_entries[dest_index] = self.entries[i].get_value();
                new_count += 1;
            }
        }

        self.entries = new_entries;
        self.entries_count = new_count;
    }
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}
