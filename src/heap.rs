use crate::builtins::core::SoxObjectPayload;
use crate::builtins::r#type::SoxType;
use crate::object::core::{Sox, SoxObject, SoxObjectInner, SoxObjectRef, SoxRef};
use std::alloc::Layout;
use std::ptr::{self, NonNull};

/// Initial threshold before first GC (in bytes)
const GC_HEAP_GROW_FACTOR: usize = 2;
const INITIAL_GC_THRESHOLD: usize = 1 * 512; // 1MB

#[derive(Debug, Default, Clone)]
pub struct GcStats {
    /// Total number of collections performed
    pub collections: usize,
    /// Total objects allocated (lifetime)
    pub total_allocations: usize,
    /// Total objects freed (lifetime)
    pub total_freed: usize,
    /// Current number of live objects
    pub live_objects: usize,
    /// Current bytes allocated
    pub bytes_allocated: usize,
}

pub struct Heap {

    /// Current bytes allocated
    bytes_allocated: usize,

    /// Threshold to trigger next GC
    next_gc_threshold: usize,

    /// GC statistics
    stats: GcStats,

    pages: Vec<Page>,
}

pub struct Page {
    mem: Box<[u8]>,
    top: usize, // offset from start of mem
}

impl Page {
    pub fn all_garbage(&self) -> bool {
        let base = self.mem.as_ptr();
        let mut offset = 0usize;
        while offset < self.top {
            let header = unsafe { &*(base.add(offset) as *const SoxObjectInner<()>) };
            if header.marked.get() {
                return false;
            }
            offset += header.gc_size;
        }
        true
    }

    pub fn clear_marks(&self) {
        let base = self.mem.as_ptr();
        let mut offset = 0usize;
        while offset < self.top {
            let header = unsafe { &*(base.add(offset) as *const SoxObjectInner<()>) };
            header.marked.set(false);
            offset += header.gc_size;
        }
    }

    /// Safely run the Drop implementation for every object currently residing on this page.
    pub fn drop_all_objects(&self) {
        let base = self.mem.as_ptr();
        let mut offset = 0usize;
        while offset < self.top {
            let header = unsafe { &*(base.add(offset) as *const SoxObjectInner<()>) };
            let obj_ref = unsafe {
                SoxObjectRef {
                    ptr: NonNull::new_unchecked(base.add(offset) as *mut SoxObject),
                }
            };
            if let Some(drop_fn) = header.typ.slots.drop {
                drop_fn(&obj_ref);
            }
            offset += header.gc_size;
        }
    }

    pub fn reset(&mut self) {
        self.top = 0;
    }

    pub fn alloc_layout(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let align_mask = layout.align() - 1;

        // round `top` up to alignment
        let start = (self.top + align_mask) & !align_mask;
        let end = start.checked_add(layout.size())?;

        if end > self.mem.len() {
            return None;
        }

        self.top = end;

        // SAFETY: start < mem.len() ensured; we keep mem alive via self
        let ptr = unsafe { self.mem.as_mut_ptr().add(start) };
        NonNull::new(ptr)
    }

    pub fn alloc_object<T: SoxObjectPayload>(
        &mut self,
        payload: T,
        typ: SoxRef<SoxType>,
        _size: usize,
    ) -> Result<SoxRef<T>, (T, SoxRef<SoxType>)> {
        let layout = Layout::new::<Sox<T>>();
        let previous_top = self.top;

        // Reserve memory FIRST — if it fails, return payload untouched
        let mem = match self.alloc_layout(layout) {
            Some(m) => m,
            None => return Err((payload, typ)),
        };

        // Actual bytes consumed = alignment padding + object size
        let actual_size = self.top - previous_top;

        let mut inner = SoxObjectInner::new(payload, typ);
        inner.gc_size = actual_size;
        let obj = Sox::<T>(*inner);

        let obj_ptr = mem.as_ptr() as *mut Sox<T>;
        unsafe {
            ptr::write(obj_ptr, obj);
            Ok(SoxRef {
                ptr: NonNull::new_unchecked(obj_ptr),
            })
        }
    }
}
const GC_PAGE_SIZE: usize = 1 * 512;
impl Heap {
    /// Create a new empty heap.
    pub fn new() -> Self {
        Heap {
            bytes_allocated: 0,
            next_gc_threshold: INITIAL_GC_THRESHOLD,
            stats: GcStats::default(),
            pages: vec![Page {
                mem: Box::new([0; GC_PAGE_SIZE]),
                top: 0,
            }],
        }
    }

    pub fn alloc<T: SoxObjectPayload>(&mut self, payload: T, typ: SoxRef<SoxType>) -> SoxRef<T> {
        let size = std::mem::size_of::<Sox<T>>();
        assert!(
            size <= GC_PAGE_SIZE,
            "Object ({size} bytes) too large for GC page ({GC_PAGE_SIZE} bytes)"
        );

        // Try last page
        let (payload, typ) = match self
            .pages
            .last_mut()
            .unwrap()
            .alloc_object(payload, typ, size)
        {
            Ok(sox_ref) => {
                self.bytes_allocated += size;
                self.stats.total_allocations += 1;
                self.stats.live_objects += 1;
                self.stats.bytes_allocated = self.bytes_allocated;
                return sox_ref;
            }
            Err(returned) => returned,
        };

        // Page was full — add a new page and allocate there
        self.pages.push(Page {
            mem: Box::new([0; GC_PAGE_SIZE]),
            top: 0,
        });

        let sox_ref = match self
            .pages
            .last_mut()
            .unwrap()
            .alloc_object(payload, typ, size)
        {
            Ok(r) => r,
            Err(_) => panic!("Failed to allocate on a fresh page"),
        };

        self.bytes_allocated += size;
        self.stats.total_allocations += 1;
        self.stats.live_objects += 1;
        self.stats.bytes_allocated = self.bytes_allocated;

        sox_ref
    }

    pub fn should_collect(&self) -> bool {
        self.bytes_allocated > self.next_gc_threshold
    }


    pub fn collect(&mut self, mark_roots: impl FnOnce(&mut dyn FnMut(SoxObjectRef))) {
        let before_bytes = self.bytes_allocated;
        let _before_pages = self.pages.len();

        // Mark phase - mark all reachable objects
        let _marked_count = self.mark(mark_roots);

        // Sweep phase-free unmarked objects
        let pages_count = self.pages.len();
        self.sweep();
        let _freed_pages_count = pages_count - self.pages.len();

        // Update threshold for next GC
        self.next_gc_threshold = self.bytes_allocated * GC_HEAP_GROW_FACTOR;
        if self.next_gc_threshold < INITIAL_GC_THRESHOLD {
            self.next_gc_threshold = INITIAL_GC_THRESHOLD;
        }

        self.stats.collections += 1;

        let freed_bytes = before_bytes.saturating_sub(self.bytes_allocated);
        let _ = freed_bytes; 
    }

    fn mark(&mut self, mark_roots: impl FnOnce(&mut dyn FnMut(SoxObjectRef))) -> usize {
        use crate::builtins::r#type::TraceFn;

        // Worklist for grey objects (discovered but not yet traced)
        let mut worklist: Vec<SoxObjectRef> = Vec::new();
        let mut marked_count: usize = 0;

        // Helper to get marked flag from a SoxObjectRef
        fn get_marked(obj: &SoxObjectRef) -> &std::cell::Cell<bool> {
            let inner = unsafe { &*(obj.ptr.as_ptr() as *const SoxObjectInner<()>) };
            &inner.marked
        }

        // Helper to get trace function from an object's type slots
        fn get_trace(obj: &SoxObjectRef) -> Option<TraceFn> {
            let inner = unsafe { &*(obj.ptr.as_ptr() as *const SoxObjectInner<()>) };
            inner.typ.slots.trace
        }

        // Helper to get type name for debug output
        fn get_type_name(obj: &SoxObjectRef) -> String {
            let inner = unsafe { &*(obj.ptr.as_ptr() as *const SoxObjectInner<()>) };
            inner
                .typ
                .name
                .clone()
                .unwrap_or_else(|| "<unknown>".to_string())
        }

        // Mark roots directly — the closure calls mark_obj for each root
        let mut root_count: usize = 0;
        let mut mark_obj = |obj: SoxObjectRef| {
            let marked_cell = get_marked(&obj);
            if !marked_cell.get() {
                marked_cell.set(true);
                root_count += 1;
                // eprintln!(
                //     "🗑️  GC:   root #{}: type={}, ptr={:p}",
                //     root_count,
                //     get_type_name(&obj),
                //     obj.ptr.as_ptr()
                // );
                worklist.push(obj);
            }
        };
        mark_roots(&mut mark_obj);
        marked_count += root_count;
        // eprintln!("🗑️  GC: Marked {} root objects", root_count);

        // Process grey objects until worklist is empty
        let mut trace_count: usize = 0;
        while let Some(obj) = worklist.pop() {
            // Look up trace function from obj.typ.slots.trace
            if let Some(trace_fn) = get_trace(&obj) {
                let type_name = get_type_name(&obj);
                let _ = type_name; // Suppress unused warning
                trace_fn(&obj, &mut |child: SoxObjectRef| {
                    let child_marked = get_marked(&child);
                    if !child_marked.get() {
                        child_marked.set(true);
                        trace_count += 1;
                        marked_count += 1;
                        worklist.push(child);
                    }
                });
            }
        }
        if trace_count > 0 {
            // eprintln!("🗑️  GC: Traced {} additional child objects", trace_count);
        }

        marked_count
    }

    /// Sweep phase: free all unmarked objects.
    fn sweep(&mut self) {
        // drop individual objects' heap fields (String, Vec, etc.) first
        self.pages.retain_mut(|page| {
            if page.all_garbage() {
                // Call drop_fn for all objects on the page before freeing it
                page.drop_all_objects();

                self.bytes_allocated -= page.top;
                page.reset();
                false // remove this page
            } else {
                page.clear_marks();
                true
            }
        });
        self.stats.bytes_allocated = self.bytes_allocated;
    }

    /// Get current GC statistics.
    #[allow(dead_code)]
    pub fn stats(&self) -> &GcStats {
        &self.stats
    }

    /// Get current bytes allocated.
    #[allow(dead_code)]
    pub fn bytes_allocated(&self) -> usize {
        self.bytes_allocated
    }

    /// Get count of live objects.
    #[allow(dead_code)]
    pub fn object_count(&self) -> usize {
        self.stats.live_objects
    }
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Heap {
    fn drop(&mut self) {
        // We must manually drop the inner payloads of our objects
        // to prevent leaking any Rust heap allocations (like String, Vec wrapper)
        for page in &mut self.pages {
            page.drop_all_objects();
        }
        // At this point, the inner fields are cleanly dropped,
        // and Rust will now naturally drop the `page.mem` Box<[u8]> array!
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::int::SoxInt;
    use crate::builtins::r#type::SoxType;
    use crate::object::core::init_type_type;
    use std::sync::Once;

    // Ensure types are only initialized once across all tests
    static INIT: Once = Once::new();
    static mut TYPE_TYPE: Option<SoxRef<SoxType>> = None;
    static mut INT_TYPE: Option<SoxRef<SoxType>> = None;

    fn get_int_type() -> SoxRef<SoxType> {
        INIT.call_once(|| {
            let type_type = init_type_type();
            let int_type = SoxRef::new_ref(
                SoxType {
                    base: None,
                    methods: Default::default(),
                    slots: Default::default(),
                    attributes: Default::default(),
                    name: Some("int".to_owned()),
                },
                type_type.clone(),
            );
            unsafe {
                TYPE_TYPE = Some(type_type);
                INT_TYPE = Some(int_type);
            }
        });
        unsafe { INT_TYPE.clone().unwrap() }
    }

    #[test]
    fn test_heap_alloc() {
        let mut heap = Heap::new();
        let int_type = get_int_type();

        let obj = heap.alloc(SoxInt { value: 42 }, int_type);

        assert_eq!(heap.object_count(), 1);
        assert!(heap.bytes_allocated() > 0);

        // Verify we can read the value back - SoxRef<T> dereferences to give access
        assert_eq!(obj.value, 42);
    }

    #[test]
    fn test_heap_collect_frees_unreachable() {
        let mut heap = Heap::new();
        let int_type = get_int_type();

        // Allocate some objects
        let _obj1 = heap.alloc(SoxInt { value: 1 }, int_type.clone());
        let _obj2 = heap.alloc(SoxInt { value: 2 }, int_type.clone());
        let obj3 = heap.alloc(SoxInt { value: 3 }, int_type);

        assert_eq!(heap.object_count(), 3);
        let bytes_before = heap.bytes_allocated();

        // Only obj3 is a root - others should be collected
        let obj3_ref = SoxObjectRef::from(obj3);
        heap.collect(|mark| {
            mark(obj3_ref);
        });

        assert_eq!(heap.object_count(), 1);
        assert!(heap.bytes_allocated() < bytes_before);
        assert_eq!(heap.stats().total_freed, 2);
    }

    #[test]
    fn test_heap_keeps_roots() {
        let mut heap = Heap::new();
        let int_type = get_int_type();

        let obj1 = heap.alloc(SoxInt { value: 100 }, int_type.clone());
        let obj2 = heap.alloc(SoxInt { value: 200 }, int_type);

        // Both are roots
        let obj1_ref = SoxObjectRef::from(obj1.clone());
        let obj2_ref = SoxObjectRef::from(obj2.clone());
        heap.collect(|mark| {
            mark(obj1_ref);
            mark(obj2_ref);
        });

        assert_eq!(heap.object_count(), 2);
        assert_eq!(heap.stats().total_freed, 0);

        // Verify values are still accessible - SoxRef<T> dereferences directly
        assert_eq!(obj1.value, 100);
        assert_eq!(obj2.value, 200);
    }

    #[test]
    fn test_gc_stats() {
        let mut heap = Heap::new();
        let int_type = get_int_type();

        for n in 0..10 {
            let _obj = heap.alloc(SoxInt { value: n }, int_type.clone());
        }

        assert_eq!(heap.stats().total_allocations, 10);
        assert_eq!(heap.stats().live_objects, 10);

        // Collect with no roots
        heap.collect(|_mark| {});

        assert_eq!(heap.stats().collections, 1);
        assert_eq!(heap.stats().total_freed, 10);
        assert_eq!(heap.stats().live_objects, 0);
    }

    #[test]
    fn test_multiple_collections() {
        let mut heap = Heap::new();
        let int_type = get_int_type();

        // First batch
        let keeper = heap.alloc(SoxInt { value: 999 }, int_type.clone());
        for n in 0..5 {
            let _obj = heap.alloc(SoxInt { value: n }, int_type.clone());
        }

        let keeper_ref = SoxObjectRef::from(keeper.clone());
        heap.collect(|mark| {
            mark(keeper_ref);
        });
        assert_eq!(heap.object_count(), 1);
        assert_eq!(heap.stats().total_freed, 5);

        // Second batch
        for n in 0..3 {
            let _obj = heap.alloc(SoxInt { value: n }, int_type.clone());
        }

        let keeper_ref2 = SoxObjectRef::from(keeper.clone());
        heap.collect(|mark| {
            mark(keeper_ref2);
        });
        assert_eq!(heap.object_count(), 1);
        assert_eq!(heap.stats().collections, 2);
        assert_eq!(heap.stats().total_freed, 8);

        // Keeper should still be valid - SoxRef<T> dereferences directly
        assert_eq!(keeper.value, 999);
    }
}
