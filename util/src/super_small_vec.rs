use std::convert::TryFrom;
use std::{
    hash::Hash,
    mem::{ManuallyDrop, MaybeUninit},
    ops::{Deref, DerefMut},
};

pub struct SuperSmallVec<T, const MAX: usize> {
    size: u8,
    data: [MaybeUninit<T>; MAX],
}

impl<T: Clone, const MAX: usize> Clone for SuperSmallVec<T, MAX> {
    fn clone(&self) -> Self {
        let mut clone = Self::new();

        for i in 0..self.size as usize {
            clone.data[i].write(self[i].clone());
        }
        clone.size = self.size;

        clone
    }
}

impl<T, const MAX: usize> SuperSmallVec<T, MAX> {
    pub fn new() -> Self {
        // FIXME: this is kind of lame but I cannot find a nice way to stop a user from using
        // a max size larger than 256 during compile time
        // #![feature(generic_const_exprs)]
        if MAX > 256 {
            panic!("Cannot have more than 256 values in SuperSmallVec");
        }
        Self {
            size: 0,
            data: unsafe { MaybeUninit::<[MaybeUninit<T>; MAX]>::uninit().assume_init() },
        }
    }

    pub fn push(&mut self, data: T) {
        if (self.size as usize) < MAX {
            self.size = self
                .size
                .checked_add(1)
                .expect("Cannot hold more than 256 values in SmallSlice");
            self.data[self.size as usize - 1].write(data);
        } else {
            panic!("Cannot hold more than {} values in SmallSlice", MAX);
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.size > 1 {
            let val = unsafe { self.data[self.size as usize].assume_init_read() };
            self.size -= 1;
            Some(val)
        } else {
            None
        }
    }
}

unsafe impl<T: Send, const MAX: usize> Send for SuperSmallVec<T, MAX> {}
unsafe impl<T: Sync, const MAX: usize> Sync for SuperSmallVec<T, MAX> {}

impl<T: PartialEq, const MAX: usize> PartialEq for SuperSmallVec<T, MAX> {
    fn eq(&self, other: &Self) -> bool {
        self.deref() == other.deref()
    }
}

impl<T: Eq, const MAX: usize> Eq for SuperSmallVec<T, MAX> {
    fn assert_receiver_is_total_eq(&self) {}
}

impl<T: Hash, const MAX: usize> Hash for SuperSmallVec<T, MAX> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.deref().hash(state);
    }
}

impl<T, const MAX: usize> Default for SuperSmallVec<T, MAX> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: std::fmt::Debug, const MAX: usize> std::fmt::Debug for SuperSmallVec<T, MAX> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.deref().iter()).finish()
    }
}

impl<T, const MAX: usize> Drop for SuperSmallVec<T, MAX> {
    fn drop(&mut self) {
        for i in 0..self.size as usize {
            unsafe { self.data[i].assume_init_drop() }
        }
    }
}

impl<T, const MAX: usize> Deref for SuperSmallVec<T, MAX> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        let slice = &self.data[..self.size as usize];
        unsafe { &*(slice as *const [MaybeUninit<T>] as *const [T]) }
    }
}

impl<T, const MAX: usize> DerefMut for SuperSmallVec<T, MAX> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let slice = &mut self.data[..self.size as usize];
        unsafe { &mut *(slice as *mut [MaybeUninit<T>] as *mut [T]) }
    }
}

impl<T, const MAX: usize> From<SuperSmallVec<T, MAX>> for Vec<T> {
    fn from(value: SuperSmallVec<T, MAX>) -> Self {
        let mut vec = Vec::with_capacity(value.len());
        // we dont drop this becuase we give away the ownership
        // of the values to the vec
        let man = ManuallyDrop::new(value);

        for i in 0..man.size as usize {
            let val = unsafe { man.data[i].assume_init_read() };
            vec.push(val);
        }
        vec
    }
}

pub struct ExceededMaximunSize;
impl<T, const MAX: usize> TryFrom<Vec<T>> for SuperSmallVec<T, MAX> {
    type Error = ExceededMaximunSize;

    fn try_from(mut value: Vec<T>) -> Result<Self, Self::Error> {
        if value.len() > MAX {
            Err(ExceededMaximunSize)
        } else {
            let mut new = Self::new();

            for i in (0..value.len()).rev() {
                new.data[i].write(value.pop().unwrap());
            }
            new.size = value.len() as u8;

            Ok(new)
        }
    }
}
