use rfw_backend::*;
use rfw_math::*;
use std::cell::UnsafeCell;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::utils::Transform;

bitflags::bitflags! {
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[repr(transparent)]
    pub struct InstanceFlags2D: u32 {
        const TRANSFORMED = 1;
    }
}

#[derive(Debug, Clone)]
pub struct InstanceList2D {
    pub(crate) list: Arc<UnsafeCell<List2D>>,
}

/// Although sharing instances amongst multiple threads without any mitigations against data races
/// is unsafe, the performance benefits of not doing any mitigation is too great to neglect this
/// opportunity (especially with many instances).
unsafe impl Send for InstanceList2D {}
unsafe impl Sync for InstanceList2D {}

impl From<List2D> for InstanceList2D {
    fn from(l: List2D) -> Self {
        Self {
            list: Arc::new(UnsafeCell::new(l)),
        }
    }
}

impl From<InstanceList2D> for List2D {
    fn from(l: InstanceList2D) -> Self {
        l.clone_inner()
    }
}

impl<'a> From<&'a InstanceList2D> for InstancesData2D<'a> {
    fn from(list: &'a InstanceList2D) -> Self {
        Self {
            matrices: list.matrices(),
        }
    }
}

impl<'a> From<&'a mut InstanceList2D> for InstancesData2D<'a> {
    fn from(list: &'a mut InstanceList2D) -> Self {
        Self {
            matrices: list.matrices(),
        }
    }
}

impl Default for InstanceList2D {
    fn default() -> Self {
        Self {
            list: Arc::new(UnsafeCell::new(List2D::default())),
        }
    }
}

impl InstanceList2D {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        unsafe { (*self.list.get()).ptr.load(Ordering::SeqCst) }
    }

    pub fn is_empty(&self) -> bool {
        (unsafe { (*self.list.get()).ptr.load(Ordering::SeqCst) }) == 0
    }

    pub fn allocate(&mut self) -> InstanceHandle2D {
        let list = unsafe { self.list.get().as_mut().unwrap() };
        if let Some(id) = list.free_slots.pop() {
            return InstanceHandle2D {
                index: id,
                ptr: self.list.clone(),
            };
        }

        let id = list.ptr.load(Ordering::Acquire);
        if (id + 1) >= list.matrices.len() {
            self.resize((id + 1) * 4);
        }
        list.ptr.store(id + 1, Ordering::Release);
        list.flags[id] = InstanceFlags2D::all();
        list.matrices[id] = Mat4::IDENTITY;

        InstanceHandle2D {
            index: id,
            ptr: self.list.clone(),
        }
    }

    pub fn make_invalid(&mut self, handle: InstanceHandle2D) {
        let list = unsafe { self.list.get().as_mut().unwrap() };
        list.matrices[handle.index] = Mat4::ZERO;
        list.flags[handle.index] = InstanceFlags2D::all();
        list.free_slots.push(handle.index);
        list.removed.push(handle.index);
    }

    pub fn resize(&mut self, new_size: usize) {
        let list = unsafe { self.list.get().as_mut().unwrap() };
        list.matrices.resize(new_size, Mat4::ZERO);
        list.flags.resize(new_size, InstanceFlags2D::empty());
    }

    pub fn get(&self, index: usize) -> Option<InstanceHandle2D> {
        let list = unsafe { self.list.get().as_mut().unwrap() };
        if list.matrices.get(index).is_some() {
            Some(InstanceHandle2D {
                index,
                ptr: self.list.clone(),
            })
        } else {
            None
        }
    }

    pub fn matrices(&self) -> &[Mat4] {
        let list = self.list.get();
        unsafe { &(*list).matrices[0..(*list).len()] }
    }

    pub fn flags(&self) -> &[InstanceFlags2D] {
        let list = self.list.get();
        unsafe { &(*list).flags[0..(*list).len()] }
    }

    pub fn set_all_flags(&mut self, flag: InstanceFlags2D) {
        let list = self.list.get();
        let flags = unsafe { &mut (*list).flags[0..(*list).len()] };
        flags.iter_mut().for_each(|f| {
            (*f) |= flag;
        });
    }

    pub fn clone_inner(&self) -> List2D {
        unsafe { self.list.get().as_ref().unwrap() }.clone()
    }

    pub fn iter(&self) -> InstanceIterator2D {
        InstanceIterator2D {
            list: self.list.clone(),
            current: 0,
            ptr: unsafe { (*self.list.get()).len() },
        }
    }

    pub fn any_changed(&self) -> bool {
        for flag in self.flags() {
            if !flag.is_empty() {
                return true;
            }
        }

        false
    }

    pub fn reset_changed(&mut self) {
        let list = unsafe { (*self.list.get()).flags.as_mut_slice() };
        for v in list.iter_mut() {
            *v = InstanceFlags2D::empty();
        }
    }

    pub fn take_removed(&mut self) -> Vec<usize> {
        let list = unsafe { self.list.get().as_mut().unwrap() };
        let mut vec = Vec::new();
        std::mem::swap(&mut vec, &mut list.removed);
        vec
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default)]
pub struct List2D {
    matrices: Vec<Mat4>,
    flags: Vec<InstanceFlags2D>,

    ptr: AtomicUsize,
    free_slots: Vec<usize>,
    removed: Vec<usize>,
}

impl Clone for List2D {
    fn clone(&self) -> Self {
        let ptr = AtomicUsize::new(self.ptr.load(Ordering::Acquire));
        let this = Self {
            matrices: self.matrices.clone(),
            flags: self.flags.clone(),

            ptr,
            free_slots: self.free_slots.clone(),
            removed: self.removed.clone(),
        };

        self.ptr.load(Ordering::Acquire);
        this
    }
}

impl List2D {
    pub fn len(&self) -> usize {
        self.ptr.load(Ordering::Acquire)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug)]
pub struct InstanceIterator2D {
    list: Arc<UnsafeCell<List2D>>,
    current: usize,
    ptr: usize,
}

impl Clone for InstanceIterator2D {
    fn clone(&self) -> Self {
        Self {
            list: self.list.clone(),
            current: 0,
            ptr: unsafe { (*self.list.get()).ptr.load(Ordering::Acquire) },
        }
    }
}

impl Iterator for InstanceIterator2D {
    type Item = InstanceHandle2D;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.ptr {
            let index = self.current;
            self.current += 1;
            return Some(InstanceHandle2D {
                index,
                ptr: self.list.clone(),
            });
        }

        None
    }
}

#[derive(Debug)]
pub struct InstanceHandle2D {
    index: usize,
    ptr: Arc<UnsafeCell<List2D>>,
}

unsafe impl Send for InstanceHandle2D {}
unsafe impl Sync for InstanceHandle2D {}

impl InstanceHandle2D {
    #[inline]
    pub fn set_matrix(&mut self, matrix: Mat4) {
        let list = unsafe { self.ptr.get().as_mut().unwrap() };
        list.matrices[self.index] = matrix;
        list.flags[self.index] |= InstanceFlags2D::TRANSFORMED;
    }

    #[inline]
    pub fn get_transform(&mut self) -> Transform<Self> {
        let (scale, rotation, translation) = self.get_matrix().to_scale_rotation_translation();

        Transform {
            translation,
            rotation,
            scale,
            handle: self,
            changed: false,
        }
    }

    #[inline]
    pub fn get_matrix(&self) -> Mat4 {
        unsafe { (*self.ptr.get()).matrices[self.index] }
    }

    #[inline]
    pub fn get_flags(&self) -> InstanceFlags2D {
        unsafe { (*self.ptr.get()).flags[self.index] }
    }

    #[inline]
    pub fn get_id(&self) -> usize {
        self.index
    }

    #[inline]
    pub fn transformed(&self) -> bool {
        unsafe { (*self.ptr.get()).flags[self.index].contains(InstanceFlags2D::TRANSFORMED) }
    }

    #[inline]
    pub fn make_invalid(self) {
        let list = unsafe { self.ptr.get().as_mut().unwrap() };
        list.matrices[self.index] = Mat4::ZERO;
        list.flags[self.index] = InstanceFlags2D::all();
        list.free_slots.push(self.index);
        list.removed.push(self.index);
    }

    /// # Safety
    ///
    /// There should only be a single instance of a handle at a time.
    /// Using these handles makes updating instances fast but leaves safety up to the user.
    pub unsafe fn clone_handle(&self) -> Self {
        Self {
            index: self.index,
            ptr: self.ptr.clone(),
        }
    }
}
