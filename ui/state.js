// VM State Management Layer - Handles all data fetching and state

class VMStateManager extends EventTarget {
    constructor() {
        super();
        this.vms = [];
        this.isPolling = false;
        this.pollInterval = null;
    }

    startPolling(intervalMs = 8000) {
        if (this.isPolling) return;

        this.isPolling = true;
        this.fetchVMs(); // Initial fetch

        this.pollInterval = setInterval(() => {
            this.fetchVMs();
        }, intervalMs);
    }

    stopPolling() {
        if (this.pollInterval) {
            clearInterval(this.pollInterval);
            this.pollInterval = null;
        }
        this.isPolling = false;
    }

    async fetchVMs() {
        const DEBUG = window.SafePawConfig?.DEBUG_STATE ?? true;
        const API_BASE = window.SafePawConfig?.API_BASE ?? 'http://localhost:8889';
        const fetchStartTime = Date.now();

        if (DEBUG) {
            console.log('[STATE] Fetching VMs...', { timestamp: fetchStartTime });
        }

        try {
            const response = await fetch(`${API_BASE}/vms`);
            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }

            const vmList = await response.json();

            // Fetch detailed info for each VM
            const detailedVMs = await Promise.all(
                vmList.map(async (vm) => {
                    try {
                        const infoResponse = await fetch(`${API_BASE}/vms/${vm.name}`);
                        if (infoResponse.ok) {
                            return await infoResponse.json();
                        }
                        return vm;
                    } catch (e) {
                        return vm;
                    }
                })
            );

            const fetchDuration = Date.now() - fetchStartTime;
            if (DEBUG) {
                console.log('[STATE] Fetch complete:', {
                    duration: `${fetchDuration}ms`,
                    vmCount: detailedVMs.length
                });
            }

            // Calculate diff and update state
            this.updateState(detailedVMs);
        } catch (error) {
            console.error('[STATE] Error fetching VMs:', error);
            this.updateState([]);
        }
    }

    updateState(newVMs) {
        const changes = this.calculateDiff(this.vms, newVMs);

        if (changes.added.length > 0 || changes.removed.length > 0 || changes.updated.length > 0) {
            this.vms = newVMs;

            // Emit granular events
            if (changes.added.length > 0) {
                this.dispatchEvent(new CustomEvent('vmsAdded', { detail: changes.added }));
            }
            if (changes.removed.length > 0) {
                this.dispatchEvent(new CustomEvent('vmsRemoved', { detail: changes.removed }));
            }
            if (changes.updated.length > 0) {
                this.dispatchEvent(new CustomEvent('vmsUpdated', { detail: changes.updated }));
            }

            // Emit general state change event
            this.dispatchEvent(new CustomEvent('stateChanged', { detail: { vms: this.vms, changes } }));
        }
    }

    calculateDiff(oldVMs, newVMs) {
        const added = [];
        const removed = [];
        const updated = [];

        const oldMap = new Map(oldVMs.map(vm => [vm.name, vm]));
        const newMap = new Map(newVMs.map(vm => [vm.name, vm]));

        // Find added and updated VMs
        for (const [name, newVM] of newMap) {
            if (!oldMap.has(name)) {
                added.push(newVM);
            } else {
                const oldVM = oldMap.get(name);
                if (this.hasVMChanged(oldVM, newVM)) {
                    updated.push(newVM);
                }
            }
        }

        // Find removed VMs
        for (const [name, oldVM] of oldMap) {
            if (!newMap.has(name)) {
                removed.push(oldVM);
            }
        }

        return { added, removed, updated };
    }

    hasVMChanged(oldVM, newVM) {
        // Check relevant fields that would affect rendering
        return oldVM.state !== newVM.state ||
               oldVM.memory_used !== newVM.memory_used ||
               oldVM.memory_total !== newVM.memory_total ||
               oldVM.disk_used !== newVM.disk_used ||
               oldVM.disk_total !== newVM.disk_total;
    }

    getVMs() {
        return this.vms;
    }
}
