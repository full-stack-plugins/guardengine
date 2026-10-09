use guardengine::integration::{FactBudget, MAX_ARTIFACT_BYTES};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
struct Observed;
static ACTIVE: AtomicBool = AtomicBool::new(false);
static MAX: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Observed {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ACTIVE.load(Ordering::Relaxed) {
            MAX.fetch_max(layout.size(), Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if ACTIVE.load(Ordering::Relaxed) {
            MAX.fetch_max(size, Ordering::Relaxed);
        }
        unsafe { System.realloc(ptr, layout, size) }
    }
}
#[global_allocator]
static ALLOC: Observed = Observed;
#[test]
fn oversized_source_parts_do_not_allocate_joined_string() {
    let field = "x".repeat(MAX_ARTIFACT_BYTES / 2);
    let mut budget = FactBudget::new();
    ACTIVE.store(true, Ordering::SeqCst);
    let result = budget.push_relation_with_source_parts("s", "p", "o", &[&field, &field]);
    ACTIVE.store(false, Ordering::SeqCst);
    assert!(result.is_err());
    assert!(
        MAX.load(Ordering::SeqCst) < 1024 * 1024,
        "large allocation before rejection"
    );
}
