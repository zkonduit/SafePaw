// Audio Manager - Handles background music
class AudioManager {
    constructor() {
        this.bgm = null;
        this.isMuted = false;
        this.isLoaded = false;

        // Try to restore mute state from localStorage
        const savedMuteState = localStorage.getItem('safepaw-muted');
        if (savedMuteState !== null) {
            this.isMuted = savedMuteState === 'true';
        }
    }

    async loadMusic() {
        try {
            const assetsPath = window.location.protocol + '//' + window.location.hostname + ':8888/assets';
            const bgmPath = `${assetsPath}/music/bgm.mp3`;

            this.bgm = new Audio(bgmPath);
            this.bgm.loop = true;
            this.bgm.volume = 0.5; // Set to 50% volume

            // Apply mute state
            this.bgm.muted = this.isMuted;

            // Wait for the audio to be ready
            await new Promise((resolve, reject) => {
                this.bgm.addEventListener('canplaythrough', resolve, { once: true });
                this.bgm.addEventListener('error', reject, { once: true });
            });

            this.isLoaded = true;
            console.log('Background music loaded successfully');

            return true;
        } catch (error) {
            console.error('Error loading background music:', error);
            return false;
        }
    }

    play() {
        if (this.bgm && this.isLoaded) {
            // Handle autoplay restrictions by catching promise rejections
            const playPromise = this.bgm.play();
            if (playPromise !== undefined) {
                playPromise.catch(error => {
                    console.warn('Autoplay prevented. Music will play after user interaction:', error);
                });
            }
        }
    }

    pause() {
        if (this.bgm) {
            this.bgm.pause();
        }
    }

    toggle() {
        this.isMuted = !this.isMuted;

        if (this.bgm) {
            if (this.isMuted) {
                this.pause();
            } else {
                this.play();
            }
        }

        // Save mute state to localStorage
        localStorage.setItem('safepaw-muted', this.isMuted.toString());

        return this.isMuted;
    }

    getState() {
        return this.isMuted;
    }
}
