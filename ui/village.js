class SafePawVillage {
    constructor(stateManager) {
        this.app = null;
        this.stateManager = stateManager;
        this.audioManager = new AudioManager();
        this.houses = new Map(); // Map of VM name -> house data
        this.tickerCallbacks = new Map(); // Track ticker callbacks for cleanup
        this.isInitialized = false;

        // Subscribe to state changes
        this.stateManager.addEventListener('vmsAdded', (e) => this.onVMsAdded(e.detail));
        this.stateManager.addEventListener('vmsRemoved', (e) => this.onVMsRemoved(e.detail));
        this.stateManager.addEventListener('vmsUpdated', (e) => this.onVMsUpdated(e.detail));
        this.stateManager.addEventListener('stateChanged', (e) => this.onStateChanged(e.detail));
    }

    async init() {
        // Create PixiJS application (full screen)
        this.app = new PIXI.Application();
        await this.app.init({
            width: window.innerWidth,
            height: window.innerHeight,
            backgroundColor: 0x87CEEB, // Sky blue - calming color
            antialias: true,
            resolution: window.devicePixelRatio || 1,
            resizeTo: window,
            eventMode: 'static',
            eventFeatures: {
                move: true,
                globalMove: true,
                click: true,
                wheel: true,
            },
        });

        const canvas = this.app.canvas;
        canvas.style.cursor = 'grab';
        canvas.style.touchAction = 'none'; // Prevent default touch behaviors
        document.getElementById('game-container').appendChild(canvas);

        // Create world container
        this.world = new PIXI.Container();
        this.app.stage.addChild(this.world);

        // Create layers FIRST (before drag controls that reference them)
        this.groundLayer = new PIXI.Container();
        this.buildingLayer = new PIXI.Container();
        this.animalLayer = new PIXI.Container();
        this.uiLayer = new PIXI.Container(); // NEW: Interactive UI elements (gear buttons, etc)
        this.effectsLayer = new PIXI.Container();

        // Make ground layer non-interactive so it doesn't block drag events
        this.groundLayer.eventMode = 'none';
        this.groundLayer.interactiveChildren = false;

        // Buildings and animals should also not block dragging
        this.buildingLayer.eventMode = 'none';
        this.buildingLayer.interactiveChildren = false;
        this.animalLayer.eventMode = 'none';
        this.animalLayer.interactiveChildren = false;

        // UI layer should allow interaction but not block dragging
        this.uiLayer.eventMode = 'passive';
        this.uiLayer.interactiveChildren = true; // Allow gear buttons to be clicked

        this.world.addChild(this.groundLayer);
        this.world.addChild(this.buildingLayer);
        this.world.addChild(this.animalLayer);
        this.world.addChild(this.uiLayer); // UI layer on top
        this.world.addChild(this.effectsLayer);

        // Add drag/pan controls (AFTER layers are created)
        this.setupDragControls();

        // Load assets (including audio)
        await this.loadAssets();
        await this.audioManager.loadMusic();

        // Create the village
        this.createGround();

        // Initialize village with current state
        const currentVMs = this.stateManager.getVMs();
        this.initializeVillage(currentVMs);

        // Hide loading screen
        document.getElementById('loading').style.display = 'none';
        document.getElementById('info-panel').style.display = 'block';

        // Setup music toggle button
        this.setupMusicToggle();

        // Setup launch VM button
        this.setupLaunchVMButton();

        // Start background music if not muted
        if (!this.audioManager.getState()) {
            this.audioManager.play();
        }

        // Start animation loop
        this.app.ticker.add(() => this.update());

        // Start state polling (state manager handles this now)
        this.stateManager.startPolling(5000);

        this.isInitialized = true;
    }

    setupMusicToggle() {
        const toggleButton = document.getElementById('mute-button');

        // Set initial button state
        this.updateMusicButtonIcon(toggleButton);

        // Add click handler
        toggleButton.addEventListener('click', () => {
            this.audioManager.toggle();
            this.updateMusicButtonIcon(toggleButton);
        });
    }

    updateMusicButtonIcon(button) {
        const isMuted = this.audioManager.getState();
        button.textContent = isMuted ? '🔇' : '🔊';
        button.title = isMuted ? 'Play music' : 'Stop music';
    }

    setupLaunchVMButton() {
        const launchButton = document.getElementById('launch-vm-button');
        if (!launchButton) {
            console.warn('Launch VM button not found');
            return;
        }

        launchButton.addEventListener('click', () => {
            this.handleLaunchVM();
        });
    }

    async handleLaunchVM() {
        // Generate a unique VM name
        const timestamp = Date.now();
        const vmName = `safepaw-${timestamp}`;

        // Confirm with user
        const confirmed = confirm(`Launch a new VM named "${vmName}"?\n\nThis may take a few minutes.`);
        if (!confirmed) {
            return;
        }

        try {
            console.log(`Launching VM: ${vmName}`);

            const response = await fetch('http://localhost:8889/vms', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ name: vmName }),
            });

            if (!response.ok) {
                const error = await response.json();
                throw new Error(error.error || 'Failed to launch VM');
            }

            const result = await response.json();
            console.log('VM launched successfully:', result);
            alert(`VM "${vmName}" is being launched! It will appear in the village shortly.`);
        } catch (error) {
            console.error('Error launching VM:', error);
            alert(`Failed to launch VM: ${error.message}`);
        }
    }

    showVMMenu(vmName, gearButton) {
        console.log(`Showing menu for VM: ${vmName}`);

        // Close any existing menu
        this.closeVMMenu();

        // Create menu container
        const menu = document.createElement('div');
        menu.id = 'vm-menu';
        menu.style.position = 'fixed';
        menu.style.background = 'rgba(255, 255, 255, 0.98)';
        menu.style.border = '2px solid #667eea';
        menu.style.borderRadius = '12px';
        menu.style.padding = '8px';
        menu.style.boxShadow = '0 4px 20px rgba(0, 0, 0, 0.3)';
        menu.style.zIndex = '1000';
        menu.style.minWidth = '150px';

        // Position menu near the gear button
        // Convert gear button world position to screen position
        const canvas = this.app.canvas;
        const rect = canvas.getBoundingClientRect();

        // Get gear button's global (screen) position
        const gearGlobalPos = gearButton.toGlobal(new PIXI.Point(0, 0));

        console.log(`🔧 [MENU] Positioning menu for ${vmName}:`, {
            gearButtonWorld: { x: gearButton.x, y: gearButton.y },
            gearButtonScreen: { x: gearGlobalPos.x, y: gearGlobalPos.y },
            canvasRect: { left: rect.left, top: rect.top }
        });

        const menuX = rect.left + gearGlobalPos.x;
        const menuY = rect.top + gearGlobalPos.y + 40; // 40px below gear button
        menu.style.left = `${menuX}px`;
        menu.style.top = `${menuY}px`;

        // Add Delete option
        const deleteOption = document.createElement('div');
        deleteOption.textContent = '🗑️ Delete VM';
        deleteOption.style.padding = '10px 16px';
        deleteOption.style.cursor = 'pointer';
        deleteOption.style.borderRadius = '8px';
        deleteOption.style.transition = 'background 0.2s ease';
        deleteOption.style.color = '#dc2626';
        deleteOption.style.fontWeight = '500';

        deleteOption.addEventListener('mouseenter', () => {
            deleteOption.style.background = 'rgba(239, 68, 68, 0.1)';
        });
        deleteOption.addEventListener('mouseleave', () => {
            deleteOption.style.background = 'transparent';
        });
        deleteOption.addEventListener('click', () => {
            this.closeVMMenu();
            this.handleDeleteVM(vmName);
        });

        menu.appendChild(deleteOption);
        document.body.appendChild(menu);

        // Close menu when clicking outside
        const closeHandler = (e) => {
            if (!menu.contains(e.target)) {
                this.closeVMMenu();
                document.removeEventListener('click', closeHandler);
            }
        };
        setTimeout(() => {
            document.addEventListener('click', closeHandler);
        }, 100);
    }

    closeVMMenu() {
        const existingMenu = document.getElementById('vm-menu');
        if (existingMenu) {
            existingMenu.remove();
        }
    }

    async handleDeleteVM(vmName) {
        // Confirm with user
        const confirmed = confirm(`Are you sure you want to delete VM "${vmName}"?\n\nThis action cannot be undone.`);
        if (!confirmed) {
            return;
        }

        try {
            console.log(`Deleting VM: ${vmName}`);

            const response = await fetch(`http://localhost:8889/vms/${vmName}`, {
                method: 'DELETE',
            });

            if (!response.ok) {
                const error = await response.json();
                throw new Error(error.error || 'Failed to delete VM');
            }

            const result = await response.json();
            console.log('VM deleted successfully:', result);
            alert(`VM "${vmName}" has been deleted.`);
        } catch (error) {
            console.error('Error deleting VM:', error);
            alert(`Failed to delete VM: ${error.message}`);
        }
    }

    async loadAssets() {
        // Load textures from the assets folder
        const assetsPath = window.location.protocol + '//' + window.location.hostname + ':8888/assets';

        try {
            // Load tiles
            console.log('Loading assets...');

            const grassPath = `${assetsPath}/tiles/grass.png`;
            console.log('Grass path:', grassPath);
            PIXI.Assets.add({ alias: 'grass', src: grassPath });

            const woodHousePath = `${assetsPath}/tiles/wood_house_walls.png`;
            console.log('Wood house path:', woodHousePath);
            PIXI.Assets.add({ alias: 'wood_house', src: woodHousePath });

            const doorPath = `${assetsPath}/tiles/door.png`;
            console.log('Door path:', doorPath);
            PIXI.Assets.add({ alias: 'door', src: doorPath });

            const chickenPath = `${assetsPath}/character/chicken.png`;
            console.log('Chicken path:', chickenPath);
            PIXI.Assets.add({ alias: 'chicken', src: chickenPath });

            const cowPath = `${assetsPath}/character/cow.png`;
            console.log('Cow path:', cowPath);
            PIXI.Assets.add({ alias: 'cow', src: cowPath });

            const plantsPath = `${assetsPath}/tiles/plants.png`;
            console.log('Plants path:', plantsPath);
            PIXI.Assets.add({ alias: 'plants', src: plantsPath });

            const waterPath = `${assetsPath}/tiles/water.png`;
            console.log('Water path:', waterPath);
            PIXI.Assets.add({ alias: 'water', src: waterPath });

            await PIXI.Assets.load(['grass', 'wood_house', 'chicken', 'cow', 'plants', 'water']);
        } catch (error) {
            console.error('Error loading assets:', error);
        }
    }

    setupDragControls() {
        // Debug flag - set to true to enable logging
        const DEBUG = window.SafePawConfig?.DEBUG_DRAG ?? false;

        // Enable interactive mode on the world container
        this.world.eventMode = 'passive';
        this.world.interactiveChildren = true; // CHANGED: Allow UI layer children to receive events

        let dragData = null;
        let currentPointer = null;
        let eventCounter = 0;

        // Use the app.stage for global event handling to avoid missing events
        this.app.stage.eventMode = 'static';
        this.app.stage.hitArea = this.app.screen;

        const onPointerDown = (event) => {
            eventCounter++;
            if (DEBUG) {
                console.log(`[${eventCounter}] POINTER DOWN:`, {
                    timestamp: Date.now(),
                    position: { x: event.global.x, y: event.global.y },
                    target: event.target?.constructor?.name || 'unknown',
                    targetEventMode: event.target?.eventMode,
                    button: event.button,
                    pointerType: event.pointerType,
                    existingDragData: !!dragData
                });
            }

            // Check if target is interactive (like a gear button) - don't start drag
            if (event.target && event.target !== this.app.stage && event.target.eventMode !== 'none') {
                console.log(`🔧 [DRAG] Skipping drag - clicked on interactive element:`, {
                    target: event.target?.constructor?.name,
                    eventMode: event.target?.eventMode
                });
                return; // Don't start dragging
            }

            // Start drag on any pointer down since all children are non-interactive
            dragData = {
                startX: event.global.x,
                startY: event.global.y,
                worldStartX: this.world.x,
                worldStartY: this.world.y,
                isDragging: false,
                eventId: eventCounter
            };
            currentPointer = event.global.clone();
            this.app.renderer.canvas.style.cursor = 'grabbing';

            if (DEBUG) {
                console.log(`[${eventCounter}] Drag started:`, {
                    startPos: { x: dragData.startX, y: dragData.startY },
                    worldPos: { x: dragData.worldStartX, y: dragData.worldStartY }
                });
            }
        };

        const onPointerMove = (event) => {
            if (dragData) {
                currentPointer = event.global.clone();
                const dx = currentPointer.x - dragData.startX;
                const dy = currentPointer.y - dragData.startY;

                // Consider it a drag if moved more than 3 pixels
                if (!dragData.isDragging && (Math.abs(dx) > 3 || Math.abs(dy) > 3)) {
                    dragData.isDragging = true;
                    if (DEBUG) {
                        console.log(`[${dragData.eventId}] DRAG ACTIVATED:`, {
                            delta: { dx, dy },
                            distance: Math.sqrt(dx*dx + dy*dy)
                        });
                    }
                }

                if (dragData.isDragging) {
                    // Apply position immediately for responsiveness
                    this.world.x = dragData.worldStartX + dx;
                    this.world.y = dragData.worldStartY + dy;
                }
            }
            // Removed excessive "no drag" logging
        };

        const endDrag = (event, eventType) => {
            if (DEBUG) {
                console.log(`[${dragData?.eventId || '?'}] POINTER ${eventType}:`, {
                    timestamp: Date.now(),
                    hadDragData: !!dragData,
                    wasDragging: dragData?.isDragging,
                    position: event ? { x: event.global.x, y: event.global.y } : 'no event'
                });
            }

            if (dragData) {
                if (DEBUG) {
                    console.log(`[${dragData.eventId}] Drag ended - Distance moved:`, {
                        dx: currentPointer ? currentPointer.x - dragData.startX : 0,
                        dy: currentPointer ? currentPointer.y - dragData.startY : 0
                    });
                }
                dragData = null;
                currentPointer = null;
                this.app.renderer.canvas.style.cursor = 'grab';
            } else if (DEBUG) {
                console.warn(`${eventType} called but no dragData exists - potential issue!`);
            }
        };

        // Attach to stage for reliable event capture
        this.app.stage.on('pointerdown', onPointerDown);
        this.app.stage.on('pointermove', onPointerMove);
        this.app.stage.on('pointerup', (e) => endDrag(e, 'UP'));
        this.app.stage.on('pointerupoutside', (e) => endDrag(e, 'UPOUTSIDE'));
        this.app.stage.on('pointercancel', (e) => endDrag(e, 'CANCEL'));

        // Also add to canvas for fallback
        const canvas = this.app.renderer.canvas;

        // Prevent context menu on right-click (helps with dragging)
        canvas.addEventListener('contextmenu', (e) => e.preventDefault());

        // Add native event listeners for debugging
        if (DEBUG) {
            canvas.addEventListener('mousedown', (e) => {
                console.log('NATIVE MOUSEDOWN:', {
                    x: e.clientX,
                    y: e.clientY,
                    button: e.button,
                    buttons: e.buttons
                });
            });

            canvas.addEventListener('mouseup', (e) => {
                console.log('NATIVE MOUSEUP:', {
                    x: e.clientX,
                    y: e.clientY,
                    button: e.button,
                    buttons: e.buttons
                });
            });

            // Track native mousemove only to detect if dragging happens without dragData
            let lastMoveWithButtonPressed = 0;
            canvas.addEventListener('mousemove', (e) => {
                if (e.buttons > 0 && !dragData) {
                    // Only log if we're getting native drag events but PixiJS dragData is missing
                    const now = Date.now();
                    if (now - lastMoveWithButtonPressed > 100) { // Throttle to every 100ms
                        console.warn('⚠️ NATIVE MOUSEMOVE with button pressed but NO dragData!', {
                            x: e.clientX,
                            y: e.clientY,
                            buttons: e.buttons
                        });
                        lastMoveWithButtonPressed = now;
                    }
                }
            });

            // Log when stage event mode changes
            console.log('Stage setup:', {
                stageEventMode: this.app.stage.eventMode,
                worldEventMode: this.world.eventMode,
                worldInteractiveChildren: this.world.interactiveChildren,
                groundLayerEventMode: this.groundLayer.eventMode,
                buildingLayerEventMode: this.buildingLayer.eventMode
            });
        }
    }

    createGround() {
        // ============================================================================
        // TILE SIZE CONFIGURATION - Change these values to test different sizes
        // ============================================================================
        const SPRITESHEET_TILE_SIZE = 16; // The actual tile size in the grass.png spritesheet (try 16, 32, or 64)
        const RENDER_TILE_SIZE = 32;      // The size to render tiles on screen (scaled up for visibility)

        // Store these for use in createGrassTile
        this.spritesheetTileSize = SPRITESHEET_TILE_SIZE;
        this.tileSize = RENDER_TILE_SIZE;

        console.log('=== TILE CONFIG ===', {
            spritesheetTileSize: SPRITESHEET_TILE_SIZE,
            renderTileSize: RENDER_TILE_SIZE
        });

        // Define grass tile variations from the spritesheet
        // Border tiles: Plain grass for clean edges (3x3 grid from rows 0-2, cols 0-2)
        // These are organized by position for proper border rendering
        this.borderGrassTiles = {
            topLeft:     { x: 0 * SPRITESHEET_TILE_SIZE, y: 0 * SPRITESHEET_TILE_SIZE },     // [0,0]
            topMiddle:   { x: 1 * SPRITESHEET_TILE_SIZE, y: 0 * SPRITESHEET_TILE_SIZE },     // [1,0]
            topRight:    { x: 2 * SPRITESHEET_TILE_SIZE, y: 0 * SPRITESHEET_TILE_SIZE },     // [2,0]
            middleLeft:  { x: 0 * SPRITESHEET_TILE_SIZE, y: 1 * SPRITESHEET_TILE_SIZE },     // [0,1]
            middleRight: { x: 2 * SPRITESHEET_TILE_SIZE, y: 1 * SPRITESHEET_TILE_SIZE },     // [2,1]
            bottomLeft:  { x: 0 * SPRITESHEET_TILE_SIZE, y: 2 * SPRITESHEET_TILE_SIZE },     // [0,2]
            bottomMiddle:{ x: 1 * SPRITESHEET_TILE_SIZE, y: 2 * SPRITESHEET_TILE_SIZE },     // [1,2]
            bottomRight: { x: 2 * SPRITESHEET_TILE_SIZE, y: 2 * SPRITESHEET_TILE_SIZE },     // [2,2]
        };

        // Center tiles: Textured grass for visual interest
        this.centerGrassVariations = [
            { x: 1 * SPRITESHEET_TILE_SIZE, y: 1 * SPRITESHEET_TILE_SIZE },     // Middle plain grass [1,1]

            // Dark texture variations (row 6, assuming rows 0-5 are other tiles)
            { x: 0 * SPRITESHEET_TILE_SIZE, y: 6 * SPRITESHEET_TILE_SIZE },     // Dark texture [0,6]
            { x: 1 * SPRITESHEET_TILE_SIZE, y: 6 * SPRITESHEET_TILE_SIZE },     // Dark texture [1,6]
            { x: 2 * SPRITESHEET_TILE_SIZE, y: 6 * SPRITESHEET_TILE_SIZE },     // Dark texture [2,6]
            { x: 3 * SPRITESHEET_TILE_SIZE, y: 6 * SPRITESHEET_TILE_SIZE },     // Dark texture [3,6]
            { x: 4 * SPRITESHEET_TILE_SIZE, y: 6 * SPRITESHEET_TILE_SIZE },     // Dark texture [4,6]

            // Light texture variations (row 7)
            { x: 0 * SPRITESHEET_TILE_SIZE, y: 7 * SPRITESHEET_TILE_SIZE },     // Light texture [0,7]
            { x: 1 * SPRITESHEET_TILE_SIZE, y: 7 * SPRITESHEET_TILE_SIZE },     // Light texture [1,7]
            { x: 2 * SPRITESHEET_TILE_SIZE, y: 7 * SPRITESHEET_TILE_SIZE },     // Light texture [2,7]
            { x: 3 * SPRITESHEET_TILE_SIZE, y: 7 * SPRITESHEET_TILE_SIZE },     // Light texture [3,7]
            { x: 4 * SPRITESHEET_TILE_SIZE, y: 7 * SPRITESHEET_TILE_SIZE },     // Light texture [4,7]
            { x: 5 * SPRITESHEET_TILE_SIZE, y: 7 * SPRITESHEET_TILE_SIZE },     // Light texture [5,7]
        ];

        // Create a large area (3x screen size for smooth scrolling)
        const tilesX = Math.ceil(this.app.screen.width / RENDER_TILE_SIZE) * 3;
        const tilesY = Math.ceil(this.app.screen.height / RENDER_TILE_SIZE) * 3;

        const startX = -Math.ceil(this.app.screen.width / RENDER_TILE_SIZE);
        const startY = -Math.ceil(this.app.screen.height / RENDER_TILE_SIZE);

        console.log('=== Creating ground tiles ===', {
            renderTileSize: RENDER_TILE_SIZE,
            spritesheetTileSize: SPRITESHEET_TILE_SIZE,
            tilesX,
            tilesY,
            totalTiles: tilesX * tilesY
        });

        for (let y = 0; y < tilesY; y++) {
            for (let x = 0; x < tilesX; x++) {
                // Determine border position
                const isTopRow = (y === 0);
                const isBottomRow = (y === tilesY - 1);
                const isLeftCol = (x === 0);
                const isRightCol = (x === tilesX - 1);
                const isBorder = isTopRow || isBottomRow || isLeftCol || isRightCol;

                // Calculate border position type
                let borderPosition = null;
                if (isBorder) {
                    if (isTopRow && isLeftCol) borderPosition = 'topLeft';
                    else if (isTopRow && isRightCol) borderPosition = 'topRight';
                    else if (isBottomRow && isLeftCol) borderPosition = 'bottomLeft';
                    else if (isBottomRow && isRightCol) borderPosition = 'bottomRight';
                    else if (isTopRow) borderPosition = 'topMiddle';
                    else if (isBottomRow) borderPosition = 'bottomMiddle';
                    else if (isLeftCol) borderPosition = 'middleLeft';
                    else if (isRightCol) borderPosition = 'middleRight';
                }

                const grass = this.createGrassTile(
                    (startX + x) * RENDER_TILE_SIZE,
                    (startY + y) * RENDER_TILE_SIZE,
                    isBorder,
                    borderPosition
                );
                this.groundLayer.addChild(grass);
            }
        }

        // Store tile dimensions for infinite scrolling
        this.tilesX = tilesX;
        this.tilesY = tilesY;

    }

    createGrassTile(x, y, isBorder = false, borderPosition = null) {
        const grassTexture = PIXI.Assets.get('grass');
        const grass = new PIXI.Sprite(grassTexture);

        let variation;
        if (isBorder && borderPosition) {
            // Use specific border tile based on position
            variation = this.borderGrassTiles[borderPosition];
        } else if (isBorder) {
            // Fallback: if no position specified, use a middle tile
            variation = this.borderGrassTiles.topMiddle;
        } else {
            // Center tiles: randomly select from textured variations
            variation = this.centerGrassVariations[Math.floor(Math.random() * this.centerGrassVariations.length)];
        }

        // Create texture from specific region of spritesheet using the spritesheet tile size
        const rect = new PIXI.Rectangle(
            variation.x,
            variation.y,
            this.spritesheetTileSize,
            this.spritesheetTileSize
        );
        grass.texture = new PIXI.Texture({
            source: grassTexture.source,
            frame: rect,
        });

        grass.x = x;
        grass.y = y;
        // Set render size (can be different from spritesheet size for scaling)
        grass.width = this.tileSize;
        grass.height = this.tileSize;

        return grass;
    }

    // addWaterFeature(x, y, width, height) {
    //     const water = new PIXI.Graphics();
    //     water.beginFill(0x4DB8FF, 0.6);
    //     water.drawRoundedRect(x, y, width, height, 10);
    //     water.endFill();

    //     // Gentle water animation
    //     this.app.ticker.add(() => {
    //         water.alpha = 0.5 + Math.sin(Date.now() * 0.001) * 0.1;
    //     });

    //     this.groundLayer.addChild(water);
    // }

    // Initialize village with VMs (called once during init)
    initializeVillage(vms) {
        if (vms.length === 0) {
            this.showWelcomeMessage();
            this.updateInfoPanel();
            return;
        }

        // Layout and create all VMs
        vms.forEach((vm, index) => {
            const position = this.calculateVMPosition(index);
            this.addVMToVillage(vm, position);
        });

        this.updateInfoPanel();
    }

    // Event handlers for state changes
    onVMsAdded(vms) {
        const DEBUG = window.SafePawConfig?.DEBUG_UI ?? true;
        if (DEBUG) {
            console.log('[UI] VMs added:', {
                count: vms.length,
                names: vms.map(vm => vm.name),
                timestamp: Date.now()
            });
        }

        this.hideWelcomeMessage();

        // Calculate starting index for layout
        const startIndex = this.houses.size;

        vms.forEach((vm, i) => {
            const position = this.calculateVMPosition(startIndex + i);
            this.addVMToVillage(vm, position);
        });

        this.updateInfoPanel();
    }

    onVMsRemoved(vms) {
        const DEBUG = window.SafePawConfig?.DEBUG_UI ?? true;
        if (DEBUG) {
            console.log('[UI] VMs removed:', {
                count: vms.length,
                names: vms.map(vm => vm.name),
                timestamp: Date.now()
            });
        }

        vms.forEach(vm => {
            this.removeVMFromVillage(vm.name);
        });

        // Reorganize remaining VMs
        this.reorganizeVillage();

        if (this.houses.size === 0) {
            this.showWelcomeMessage();
        }

        this.updateInfoPanel();
    }

    onVMsUpdated(vms) {
        const DEBUG = window.SafePawConfig?.DEBUG_UI ?? true;
        if (DEBUG) {
            console.log('[UI] VMs updated:', {
                count: vms.length,
                names: vms.map(vm => vm.name),
                timestamp: Date.now()
            });
        }

        vms.forEach(vm => {
            this.updateVMInVillage(vm);
        });

        this.updateInfoPanel();
    }

    onStateChanged() {
        // General state change handler (if needed)
        // Individual handlers above are more granular
    }

    calculateVMPosition(index) {
        const padding = 150;
        const spacing = 600; // Increased from 150 to 600 to prevent overlap (13x13 tiles * 32px * 1.5 scale = ~624px)
        const startX = padding;
        const startY = padding;

        const col = index % 3; // Reduced from 4 to 3 columns for better spacing
        const row = Math.floor(index / 3);

        return {
            x: startX + col * spacing,
            y: startY + row * spacing
        };
    }

    addVMToVillage(vm, position) {
        if (this.houses.has(vm.name)) {
            console.warn(`VM ${vm.name} already exists in village`);
            return;
        }

        const houseData = this.createHouse(vm, position.x, position.y);
        this.houses.set(vm.name, houseData);
    }

    removeVMFromVillage(vmName) {
        const houseData = this.houses.get(vmName);
        if (!houseData) return;

        // Clean up ticker callbacks
        this.cleanupHouseCallbacks(vmName);

        // Remove sprites
        if (houseData.house) {
            this.buildingLayer.removeChild(houseData.house);
        }
        if (houseData.nameText) {
            this.buildingLayer.removeChild(houseData.nameText);
        }
        if (houseData.gearButton) {
            this.uiLayer.removeChild(houseData.gearButton); // Remove from UI layer
        }
        if (houseData.animals) {
            houseData.animals.forEach(animal => {
                this.animalLayer.removeChild(animal);
            });
        }

        this.houses.delete(vmName);
    }

    updateVMInVillage(vm) {
        const houseData = this.houses.get(vm.name);
        if (!houseData) {
            console.warn(`VM ${vm.name} not found in village for update`);
            return;
        }

        // Update house appearance based on state
        let tintColor;
        if (vm.state === 'Running') {
            tintColor = 0xFFFFFF;
        } else if (vm.state === 'Stopped') {
            tintColor = 0xCCCCCC;
        } else {
            tintColor = 0xFFEEAA;
        }

        // Apply tint to all tiles in the house container
        if (houseData.house.children) {
            houseData.house.children.forEach(tile => {
                tile.tint = tintColor;
            });
        } else {
            // Fallback for old single-sprite houses
            houseData.house.tint = tintColor;
        }

        // Clean up old animal callbacks (keep house breathing callback)
        const callbacks = this.tickerCallbacks.get(vm.name) || [];
        const houseCallback = callbacks[0]; // First callback is always the house breathing

        // Remove all animal callbacks (everything except first)
        for (let i = 1; i < callbacks.length; i++) {
            this.app.ticker.remove(callbacks[i]);
        }

        // Reset to just house callback
        this.tickerCallbacks.set(vm.name, [houseCallback]);

        // Update animals (remove old ones, add new ones)
        if (houseData.animals) {
            houseData.animals.forEach(animal => {
                this.animalLayer.removeChild(animal);
            });
        }

        houseData.animals = [];

        if (vm.state === 'Running') {
            const animals = this.createAnimals(
                vm,
                houseData.house.x,
                houseData.house.y,
                houseData.houseSize,
                houseData.tileSize,
                houseData.houseScale
            );
            houseData.animals = animals;
        }

        // Store updated VM data
        houseData.vm = vm;
    }

    reorganizeVillage() {
        const DEBUG = window.SafePawConfig?.DEBUG_UI ?? true;
        if (DEBUG) {
            console.log('[UI] Reorganizing village:', {
                houseCount: this.houses.size,
                timestamp: Date.now()
            });
        }

        // Recalculate positions for all remaining VMs
        const vmNames = Array.from(this.houses.keys());

        vmNames.forEach((vmName, index) => {
            const houseData = this.houses.get(vmName);
            const newPosition = this.calculateVMPosition(index);

            // Smooth position transition
            if (houseData.house) {
                houseData.house.x = newPosition.x;
                houseData.house.y = newPosition.y;
            }
            if (houseData.nameText) {
                houseData.nameText.x = newPosition.x;
                houseData.nameText.y = newPosition.y + 40;
            }

            // Reposition animals
            if (houseData.animals && houseData.vm.state === 'Running') {
                // Remove old animals
                houseData.animals.forEach(animal => this.animalLayer.removeChild(animal));

                // Create new animals at new position with house parameters
                houseData.animals = this.createAnimals(
                    houseData.vm,
                    newPosition.x,
                    newPosition.y,
                    houseData.houseSize,
                    houseData.tileSize,
                    houseData.houseScale
                );
            }
        });

        // Verify event modes haven't been changed
        if (DEBUG) {
            console.log('[UI] After reorganize - event modes:', {
                groundLayerEventMode: this.groundLayer.eventMode,
                buildingLayerEventMode: this.buildingLayer.eventMode,
                animalLayerEventMode: this.animalLayer.eventMode,
                worldEventMode: this.world.eventMode,
                stageEventMode: this.app.stage.eventMode
            });
        }
    }

    showWelcomeMessage() {
        if (this.welcomeText) return; // Already showing

        this.welcomeText = new PIXI.Text('No VMs yet! Create one with:\nsafepaw vm launch <name>', {
            fontFamily: 'Arial',
            fontSize: 24,
            fill: 0x333333,
            align: 'center',
        });
        this.welcomeText.x = this.app.screen.width / 2;
        this.welcomeText.y = this.app.screen.height / 2;
        this.welcomeText.anchor.set(0.5);
        this.buildingLayer.addChild(this.welcomeText);
    }

    hideWelcomeMessage() {
        if (this.welcomeText) {
            this.buildingLayer.removeChild(this.welcomeText);
            this.welcomeText = null;
        }
    }

    cleanupHouseCallbacks(vmName) {
        const callbacks = this.tickerCallbacks.get(vmName);
        if (callbacks) {
            callbacks.forEach(callback => {
                this.app.ticker.remove(callback);
            });
            this.tickerCallbacks.delete(vmName);
        }
    }

    // Generate house tile layout dynamically based on size
    // Size is the number of tiles per side (e.g., 5 = 5x5 house, 8 = 8x8 house)
    generateHouseLayout(size) {
        const layout = [];

        // Tile mapping from spritesheet:
        // [0,0] = Left back corner
        // [1,0] = Back wall
        // [2,0] = Right back corner
        // [0,1] = Left wall
        // [1,1] = Floor/interior
        // [2,1] = Right wall
        // [0,2] = Left front corner
        // [1,2] = Front wall
        // [2,2] = Right front corner
        // [3,1] = Door (closed)

        for (let row = 0; row < size; row++) {
            for (let col = 0; col < size; col++) {
                let tile;

                // Determine tile type based on position
                if (row === 0) {
                    // Top row (back wall)
                    if (col === 0) {
                        tile = { col: 0, row: 0 }; // Left back corner
                    } else if (col === size - 1) {
                        tile = { col: 2, row: 0 }; // Right back corner
                    } else {
                        tile = { col: 1, row: 0 }; // Back wall
                    }
                } else if (row === size - 1) {
                    // Bottom row (front wall)
                    if (col === 0) {
                        tile = { col: 0, row: 2 }; // Left front corner
                    } else if (col === size - 1) {
                        tile = { col: 2, row: 2 }; // Right front corner
                    } else if (col === Math.floor(size / 2)) {
                        tile = { col: 3, row: 1 }; // Door in the center
                    } else {
                        tile = { col: 1, row: 2 }; // Front wall
                    }
                } else {
                    // Middle rows
                    if (col === 0) {
                        tile = { col: 0, row: 1 }; // Left wall
                    } else if (col === size - 1) {
                        tile = { col: 2, row: 1 }; // Right wall
                    } else {
                        tile = { col: 1, row: 1 }; // Floor/interior
                    }
                }

                layout.push(tile);
            }
        }

        return layout;
    }

    createHouse(vm, x, y) {
        // ============================================================================
        // HOUSE TILE CONFIGURATION
        // ============================================================================
        const HOUSE_TILE_SIZE = 16;        // The actual tile size in wood_house.png spritesheet
        const HOUSE_RENDER_TILE_SIZE = 32; // The size to render each house tile

        // House size based on VM resources (not animal count anymore)
        const memoryGB = (vm.memory_total || 0) / (1024 * 1024 * 1024);
        const diskGB = (vm.disk_total || 0) / (1024 * 1024 * 1024);

        // House size scales with memory - FIXED to remove random variation
        // Small VMs: 7x7 (< 1 GB memory)
        // Medium VMs: 10x10 (1-4 GB memory)
        // Large VMs: 13x13 (> 4 GB memory)
        let houseSize;
        if (memoryGB < 1) {
            houseSize = 7;
        } else if (memoryGB < 4) {
            houseSize = 10;
        } else {
            houseSize = 13;
        }

        console.log('Creating house:', {
            vm: vm.name,
            resources: { memoryGB: memoryGB.toFixed(2), diskGB: diskGB.toFixed(2) },
            houseSize: `${houseSize}x${houseSize}`,
            totalTiles: houseSize * houseSize
        });

        // Generate house layout dynamically (which sprite tiles to use)
        const houseTileLayout = this.generateHouseLayout(houseSize);

        // Create a container for the house
        const houseContainer = new PIXI.Container();

        // Build the house from tiles
        const houseTexture = PIXI.Assets.get('wood_house');

        // Render all tiles
        for (let i = 0; i < houseTileLayout.length; i++) {
            const tileInfo = houseTileLayout[i];

            // Calculate position on screen (row and column from index)
            const col = i % houseSize;
            const row = Math.floor(i / houseSize);

            const tileSprite = new PIXI.Sprite(houseTexture);

            // Extract tile from spritesheet (which tile design to use)
            const rect = new PIXI.Rectangle(
                tileInfo.col * HOUSE_TILE_SIZE,
                tileInfo.row * HOUSE_TILE_SIZE,
                HOUSE_TILE_SIZE,
                HOUSE_TILE_SIZE
            );
            tileSprite.texture = new PIXI.Texture({
                source: houseTexture.source,
                frame: rect,
            });

            // Position on screen (where to place it)
            tileSprite.x = col * HOUSE_RENDER_TILE_SIZE;
            tileSprite.y = row * HOUSE_RENDER_TILE_SIZE;
            tileSprite.width = HOUSE_RENDER_TILE_SIZE;
            tileSprite.height = HOUSE_RENDER_TILE_SIZE;

            houseContainer.addChild(tileSprite);
        }

        // Position the house container
        houseContainer.x = x;
        houseContainer.y = y;

        // Center the container dynamically based on house size
        houseContainer.pivot.set(
            (houseSize * HOUSE_RENDER_TILE_SIZE) / 2,
            (houseSize * HOUSE_RENDER_TILE_SIZE) / 2
        );

        // Fixed scale for consistency - no more random variation
        const baseScale = 1.5;
        houseContainer.scale.set(baseScale);

        // Subtle breathing animation (calming)
        const breathingSpeed = 0.0005;
        const breathingAmount = 0.02;
        houseContainer.userData = {
            baseScale: baseScale,
            time: Math.random() * Math.PI * 2
        };

        // Track ticker callback for cleanup
        const callbacks = [];
        const breathingCallback = () => {
            houseContainer.userData.time += breathingSpeed;
            const breath = Math.sin(houseContainer.userData.time) * breathingAmount;
            houseContainer.scale.set(houseContainer.userData.baseScale * (1 + breath));
        };
        this.app.ticker.add(breathingCallback);
        callbacks.push(breathingCallback);

        // VM state color tint - apply to all tiles in the container
        let tintColor;
        if (vm.state === 'Running') {
            tintColor = 0xFFFFFF; // Normal
        } else if (vm.state === 'Stopped') {
            tintColor = 0xCCCCCC; // Grayed out
        } else {
            tintColor = 0xFFEEAA; // Yellowish for other states
        }

        // Apply tint to all child sprites
        houseContainer.children.forEach(tile => {
            tile.tint = tintColor;
        });

        this.buildingLayer.addChild(houseContainer);

        // Add VM name label - positioned ABOVE the house
        const nameText = new PIXI.Text(vm.name, {
            fontFamily: 'Arial',
            fontSize: 24,  // Increased from 14 to 24
            fill: 0x333333,
            fontWeight: 'bold',
        });
        nameText.x = x;
        // Position above the house (negative Y offset)
        // House height depends on houseSize, offset by half that height plus padding
        const houseHeight = (houseSize * HOUSE_RENDER_TILE_SIZE * baseScale);
        nameText.y = y - houseHeight / 2 - 40; // Above the house
        nameText.anchor.set(0.5);
        this.buildingLayer.addChild(nameText);

        // Add gear button to the left of the house name for VM management
        // Use a container with a visible background for easier debugging
        const gearButton = new PIXI.Container();

        // Add visible background for debugging
        const gearBg = new PIXI.Graphics();
        gearBg.beginFill(0xFF0000, 0.3); // Semi-transparent red background for debugging
        gearBg.drawCircle(0, 0, 25);
        gearBg.endFill();
        gearButton.addChild(gearBg);

        // Add the gear emoji/text
        const gearText = new PIXI.Text('⚙️', {
            fontFamily: 'Arial',
            fontSize: 32,
        });
        gearText.anchor.set(0.5);
        gearButton.addChild(gearText);

        gearButton.x = nameText.x - nameText.width / 2 - 40; // Left of name
        gearButton.y = nameText.y;
        gearButton.eventMode = 'static';
        gearButton.cursor = 'pointer';
        gearButton.zIndex = 1000; // Ensure it's above other elements

        // Make it very visible for debugging
        gearButton.interactive = true;
        gearButton.buttonMode = true;
        gearButton.hitArea = new PIXI.Circle(0, 0, 50); // Much larger hit area (50px radius)

        // Extensive debug logging for gear button
        console.log(`🔧 [GEAR] Creating gear button for ${vm.name}:`, {
            position: { x: gearButton.x, y: gearButton.y },
            worldPosition: { worldX: x, worldY: y, nameY: nameText.y },
            eventMode: gearButton.eventMode,
            interactive: gearButton.interactive,
            buttonMode: gearButton.buttonMode,
            cursor: gearButton.cursor,
            zIndex: gearButton.zIndex,
            bounds: gearButton.getBounds()
        });

        // Add ALL possible event listeners with debug logs
        gearButton.on('pointerover', (event) => {
            console.log(`🔧 [GEAR] HOVER IN for ${vm.name}`, {
                target: event.target,
                currentTarget: event.currentTarget,
                position: { x: event.global.x, y: event.global.y }
            });
            gearButton.scale.set(1.2);
        });

        gearButton.on('pointerout', () => {
            console.log(`🔧 [GEAR] HOVER OUT for ${vm.name}`);
            gearButton.scale.set(1.0);
        });

        gearButton.on('pointerdown', (event) => {
            console.log(`🔧 [GEAR] POINTER DOWN for ${vm.name}`, {
                button: event.button,
                buttons: event.buttons,
                target: event.target,
                position: { x: event.global.x, y: event.global.y }
            });
        });

        gearButton.on('pointerup', () => {
            console.log(`🔧 [GEAR] POINTER UP for ${vm.name}`);
        });

        gearButton.on('pointertap', (event) => {
            console.log(`🔧 [GEAR] POINTER TAP for ${vm.name}`, {
                event,
                target: event.target,
                currentTarget: event.currentTarget
            });
            event.stopPropagation(); // Prevent event bubbling
            this.showVMMenu(vm.name, gearButton);
        });

        gearButton.on('click', (event) => {
            console.log(`🔧 [GEAR] CLICK for ${vm.name}`);
            event.stopPropagation();
            this.showVMMenu(vm.name, gearButton);
        });

        gearButton.on('tap', (event) => {
            console.log(`🔧 [GEAR] TAP for ${vm.name}`);
            event.stopPropagation();
            this.showVMMenu(vm.name, gearButton);
        });

        // Add to UI layer instead of building layer for proper interaction
        this.uiLayer.addChild(gearButton);

        // Log layer hierarchy after adding
        console.log(`🔧 [GEAR] Added to uiLayer for ${vm.name}:`, {
            uiLayerEventMode: this.uiLayer.eventMode,
            uiLayerInteractiveChildren: this.uiLayer.interactiveChildren,
            uiLayerChildren: this.uiLayer.children.length,
            worldEventMode: this.world.eventMode,
            stageEventMode: this.app.stage.eventMode,
            gearButtonParent: gearButton.parent?.constructor?.name
        });

        // Add native canvas event listener to debug if events are reaching the canvas at all
        if (!this._canvasDebugAdded) {
            this._canvasDebugAdded = true;
            const canvas = this.app.canvas;

            canvas.addEventListener('click', (e) => {
                const rect = canvas.getBoundingClientRect();
                const canvasX = e.clientX - rect.left;
                const canvasY = e.clientY - rect.top;
                console.log(`🖱️ [CANVAS] Native click at canvas coords:`, {
                    canvasX,
                    canvasY,
                    clientX: e.clientX,
                    clientY: e.clientY,
                    target: e.target
                });

                // Check if click is near gear button
                const worldPoint = this.world.toLocal(new PIXI.Point(canvasX, canvasY));
                console.log(`🖱️ [CANVAS] Click in world coords:`, {
                    worldX: worldPoint.x,
                    worldY: worldPoint.y
                });

                // Log all gear button positions for comparison
                console.log(`🖱️ [CANVAS] Gear button positions:`,
                    Array.from(this.houses.values()).map(h => ({
                        name: h.vm.name,
                        gearX: h.gearButton?.x,
                        gearY: h.gearButton?.y,
                        distance: h.gearButton ? Math.sqrt(
                            Math.pow(worldPoint.x - h.gearButton.x, 2) +
                            Math.pow(worldPoint.y - h.gearButton.y, 2)
                        ) : null
                    }))
                );
            }, true);
        }

        // Add animals representing usage (fixed 5 each, positioned at bottom-right of house)
        let animals = [];
        if (vm.state === 'Running') {
            animals = this.createAnimals(vm, x, y, houseSize, HOUSE_RENDER_TILE_SIZE, baseScale);
        }

        // Store ticker callbacks for this VM
        this.tickerCallbacks.set(vm.name, callbacks);

        // Return house data object (using houseContainer instead of house)
        return {
            house: houseContainer,
            nameText,
            gearButton,
            animals,
            vm,
            // Store house parameters for later use (when updating/reorganizing)
            houseSize,
            tileSize: HOUSE_RENDER_TILE_SIZE,
            houseScale: baseScale
        };
    }

    createAnimals(vm, houseX, houseY, houseSize, tileSize, houseScale) {
        // Animals represent resource usage
        // Chickens = memory usage (0-5 based on percentage)
        // Cows = disk usage (0-5 based on percentage)
        // Positioned to the right of the house
        const animals = [];
        const callbacks = this.tickerCallbacks.get(vm.name) || [];

        // Calculate number based on usage percentage
        const memoryPercent = (vm.memory_used || 0) / (vm.memory_total || 1);
        const diskPercent = (vm.disk_used || 0) / (vm.disk_total || 1);

        // Map percentage to 0-5 animals
        // 0-10% = 0, 10-30% = 1, 30-50% = 2, 50-70% = 3, 70-90% = 4, 90-100% = 5
        const numChickens = Math.min(Math.floor(memoryPercent * 5 + 0.5), 5);
        const numCows = Math.min(Math.floor(diskPercent * 5 + 0.5), 5);

        console.log('Creating animals:', {
            vm: vm.name,
            memory: `${(memoryPercent * 100).toFixed(1)}%`,
            disk: `${(diskPercent * 100).toFixed(1)}%`,
            chickens: numChickens,
            cows: numCows
        });

        // Calculate right side position
        // House is centered, so we need to offset from center
        const houseWidth = houseSize * tileSize * houseScale;
        const houseHeight = houseSize * tileSize * houseScale;

        // Right side of house, centered vertically
        const rightX = houseX + houseWidth / 2 + 40; // 40px padding from house
        const centerY = houseY;

        // Add chickens vertically stacked on the right side
        for (let i = 0; i < numChickens; i++) {
            const offsetY = -80 + i * 40; // Increased vertical spacing for bigger sprites

            const { chicken, callback } = this.createChicken(rightX, centerY + offsetY, i);
            animals.push(chicken);
            callbacks.push(callback);
        }

        // Add cows vertically stacked on the right side (to the right of chickens)
        for (let i = 0; i < numCows; i++) {
            const offsetX = 80; // More space to the right of chickens
            const offsetY = -100 + i * 50; // Increased vertical spacing (cows are bigger)

            const { cow, callback } = this.createCow(rightX + offsetX, centerY + offsetY, i);
            animals.push(cow);
            callbacks.push(callback);
        }

        // Update callbacks for this VM
        this.tickerCallbacks.set(vm.name, callbacks);

        return animals;
    }

    createChicken(x, y, index) {
        const chickenTexture = PIXI.Assets.get('chicken');
        const CHICKEN_FRAME_SIZE = 16;

        // Chicken spritesheet layout:
        // Row 0: [idle] [blink] [blank] [blank]
        // Row 1: [walk1] [walk2] [walk3] [walk4]

        // Create idle frame as default
        const idleFrame = new PIXI.Rectangle(0, 0, CHICKEN_FRAME_SIZE, CHICKEN_FRAME_SIZE);
        const chicken = new PIXI.Sprite(new PIXI.Texture({
            source: chickenTexture.source,
            frame: idleFrame,
        }));

        chicken.scale.set(3.0); // Bigger scale for visibility (3x from 16x16 = 48px)
        chicken.x = x;
        chicken.y = y;
        chicken.anchor.set(0.5);

        // Animation state
        chicken.userData = {
            baseY: y,
            time: Math.random() * Math.PI * 2,
            speed: 0.002 + Math.random() * 0.001,
            animFrame: 0,
            animTime: Math.random() * 100
        };

        // Callback handles both bobbing and sprite animation
        const callback = () => {
            chicken.userData.time += chicken.userData.speed;
            chicken.y = chicken.userData.baseY + Math.sin(chicken.userData.time) * 2;

            // Occasionally blink or animate
            chicken.userData.animTime++;
            if (chicken.userData.animTime > 120) { // Every ~2 seconds
                // Blink frame
                const blinkFrame = new PIXI.Rectangle(
                    1 * CHICKEN_FRAME_SIZE,
                    0,
                    CHICKEN_FRAME_SIZE,
                    CHICKEN_FRAME_SIZE
                );
                chicken.texture = new PIXI.Texture({
                    source: chickenTexture.source,
                    frame: blinkFrame,
                });

                // Reset after a short time
                setTimeout(() => {
                    const idleFrame = new PIXI.Rectangle(0, 0, CHICKEN_FRAME_SIZE, CHICKEN_FRAME_SIZE);
                    chicken.texture = new PIXI.Texture({
                        source: chickenTexture.source,
                        frame: idleFrame,
                    });
                }, 100);

                chicken.userData.animTime = 0;
            }
        };

        this.app.ticker.add(callback);
        this.animalLayer.addChild(chicken);

        return { chicken, callback };
    }

    createCow(x, y, index) {
        const cowTexture = PIXI.Assets.get('cow');
        const COW_FRAME_SIZE = 32; // Cows are 32x32 pixels (larger than chickens)

        // Cow spritesheet layout:
        // Row 0: [idle1] [blink] [idle2]
        // Row 1: [walk1] [walk2] [blank] [blank]

        // Alternate between idle1 and idle2 for variety
        const idleCol = index % 2 === 0 ? 0 : 2;
        const idleFrame = new PIXI.Rectangle(
            idleCol * COW_FRAME_SIZE,
            0,
            COW_FRAME_SIZE,
            COW_FRAME_SIZE
        );
        const cow = new PIXI.Sprite(new PIXI.Texture({
            source: cowTexture.source,
            frame: idleFrame,
        }));

        cow.scale.set(2.0); // Bigger scale for visibility (2x from 32x32 = 64px)
        cow.x = x;
        cow.y = y;
        cow.anchor.set(0.5);

        // Animation state
        cow.userData = {
            baseX: x,
            time: Math.random() * Math.PI * 2,
            speed: 0.001 + Math.random() * 0.0005,
            idleCol: idleCol, // Remember which idle frame we're using
            animTime: Math.random() * 150
        };

        // Callback handles both swaying and sprite animation
        const callback = () => {
            cow.userData.time += cow.userData.speed;
            cow.x = cow.userData.baseX + Math.sin(cow.userData.time) * 2;

            // Occasionally blink
            cow.userData.animTime++;
            if (cow.userData.animTime > 180) { // Every ~3 seconds
                // Blink frame (col 1, row 0)
                const blinkFrame = new PIXI.Rectangle(
                    1 * COW_FRAME_SIZE,
                    0,
                    COW_FRAME_SIZE,
                    COW_FRAME_SIZE
                );
                cow.texture = new PIXI.Texture({
                    source: cowTexture.source,
                    frame: blinkFrame,
                });

                // Reset back to original idle frame after a short time
                setTimeout(() => {
                    const idleFrame = new PIXI.Rectangle(
                        cow.userData.idleCol * COW_FRAME_SIZE,
                        0,
                        COW_FRAME_SIZE,
                        COW_FRAME_SIZE
                    );
                    cow.texture = new PIXI.Texture({
                        source: cowTexture.source,
                        frame: idleFrame,
                    });
                }, 150);

                cow.userData.animTime = 0;
            }
        };

        this.app.ticker.add(callback);
        this.animalLayer.addChild(cow);

        return { cow, callback };
    }

    updateInfoPanel() {
        const statsDiv = document.getElementById('vm-stats');
        const vms = this.stateManager.getVMs();

        if (vms.length === 0) {
            statsDiv.innerHTML = '<p style="color: #999; margin-top: 10px;">No VMs running. Launch one to see your village grow!</p>';
            return;
        }

        let html = '<div style="margin-top: 15px;">';
        html += `<div class="vm-stat"><span class="stat-label">Total VMs:</span><span class="stat-value">${vms.length}</span></div>`;

        const runningVMs = vms.filter(vm => vm.state === 'Running').length;
        html += `<div class="vm-stat"><span class="stat-label">Running:</span><span class="stat-value">${runningVMs}</span></div>`;

        html += '<div style="margin-top: 15px; padding-top: 15px; border-top: 2px solid #e0e0e0;">';
        html += '<p style="color: #667eea; font-weight: 600; margin-bottom: 8px;">🐔 Chickens = Memory Usage</p>';
        html += '<p style="color: #667eea; font-weight: 600;">🐄 Cows = Disk Usage</p>';
        html += '</div>';

        html += '</div>';
        statsDiv.innerHTML = html;
    }

    update() {
        // Smooth update loop
        // Additional animations and effects can be added here
    }
}
