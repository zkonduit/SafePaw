// SafePaw Village - A calming, RPG-style VM visualization
// Neuroscience-backed features:
// 1. Soft, natural colors (green/blue reduce cortisol)
// 2. Gentle animations (smooth movements calm the nervous system)
// 3. Predictable patterns (reduces cognitive load & anxiety)
// 4. Nature elements (proven to reduce stress)
// 5. Positive visual feedback (dopamine release)

// DEBUG MODE: Comprehensive logging is enabled to debug drag responsiveness
// To analyze the logs when drag becomes unresponsive:
//
// NORMAL DRAG SEQUENCE should look like:
// 1. NATIVE MOUSEDOWN
// 2. [N] POINTER DOWN
// 3. [N] Drag started
// 4. NATIVE MOUSEMOVE (dragging) - multiple times
// 5. [N] DRAG ACTIVATED
// 6. NATIVE MOUSEUP
// 7. [N] POINTER UP
// 8. [N] Drag ended
//
// PROBLEM PATTERNS TO LOOK FOR:
// A. Missing POINTER events but NATIVE events present = PixiJS event system blocked
// B. "POINTER UP called but no dragData exists" = drag state lost prematurely
// C. POINTER DOWN but no Drag started = onPointerDown not executing
// D. [STATE] or [UI] events happening during drag = state update interfering
// E. Event mode changes during drag = layer configuration changed
// F. Multiple POINTER DOWN without POINTER UP = drag state not clearing
//
// Check the timestamp correlation between events to find timing issues

// ============================================================================
// GLOBAL CONFIGURATION
// ============================================================================

// API Configuration
const API_BASE = window.location.protocol + '//' + window.location.hostname + ':8889';

// Debug Flags
const DEBUG_STATE = true;  // State manager debug logs
const DEBUG_UI = true;     // UI/Village debug logs
const DEBUG_DRAG = false;  // Drag interaction debug logs

// Export for use in other modules
window.SafePawConfig = {
    API_BASE,
    DEBUG_STATE,
    DEBUG_UI,
    DEBUG_DRAG
};

// ============================================================================
// APPLICATION INITIALIZATION
// ============================================================================

// Initialize the village when the page loads
window.addEventListener('load', async () => {
    // Create state manager (imported from state.js)
    const stateManager = new VMStateManager();

    // Create village with state manager (imported from village.js)
    const village = new SafePawVillage(stateManager);
    await village.init();
});
