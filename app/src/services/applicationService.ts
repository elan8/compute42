import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { debug, trace, logError } from '../utils/logger';
import { unifiedEventService, EventCategory } from './unifiedEventService';

export interface InitializationStatus {
  message: string;
  progress: number;
  is_error: boolean;
  error_details?: string;
}

export interface ProjectChangeStatus {
  message: string;
  progress: number;
  is_error: boolean;
  error_details?: string;
}

export interface ProjectChangeComplete {
  project_path: string;
}

export class ApplicationService {
  private initializationStatusUnlistenFn: UnlistenFn | null = null;
  private initializationCompleteUnlistenFn: UnlistenFn | null = null;
  private projectChangeStatusUnlistenFn: UnlistenFn | null = null;
  private projectChangeCompleteUnlistenFn: UnlistenFn | null = null;
  private compute42ReadyUnlistenFn: UnlistenFn | null = null;

  constructor() {
    this.setupEventListeners();
  }

  private async setupEventListeners() {
    try {
      // Set up unified event listeners for orchestrator events
      await unifiedEventService.addEventListener(
        EventCategory.Orchestrator,
        'initialization-status',
        async (event) => {
          const payload = event.payload;
          if (payload.message && payload.progress !== undefined) {
            this.handleInitializationStatus({
              message: payload.message,
              progress: payload.progress,
              is_error: payload.is_error || false,
              error_details: payload.error_details,
            });
          }
        }
      );

      // Legacy event listeners for backward compatibility
      this.initializationStatusUnlistenFn = await listen<InitializationStatus>(
        'initialization-status',
        (event) => {
          this.handleInitializationStatus(event.payload);
        }
      );

      this.initializationCompleteUnlistenFn = await listen('initialization-complete', () => {
        this.handleInitializationComplete();
      });

      this.projectChangeStatusUnlistenFn = await listen<ProjectChangeStatus>(
        'project-change-status',
        (event) => {
          this.handleProjectChangeStatus(event.payload);
        }
      );

      this.projectChangeCompleteUnlistenFn = await listen<ProjectChangeComplete>(
        'project-change-complete',
        (event) => {
          this.handleProjectChangeComplete(event.payload);
        }
      );

      this.compute42ReadyUnlistenFn = await listen('orchestrator:startup-ready', () => {
        this.handleCompute42Ready();
      });

      await trace('ApplicationService: Event listeners set up successfully');
    } catch (error) {
      await logError('ApplicationService: Failed to set up event listeners', error);
    }
  }

  /**
   * Signal to backend that frontend is ready for initialization
   */
  async handleStartupComplete(): Promise<void> {
    try {
      // Note: frontend_ready is now handled by the backend-ready handshake mechanism
    } catch (error) {
      await logError('ApplicationService: Failed to handle startup complete', error);
      throw error;
    }
  }

  /**
   * Signal to backend that project has changed
   */
  async handleProjectChange(newProjectPath: string): Promise<void> {
    try {
      await invoke('project_changed', { projectPath: newProjectPath });
    } catch (error) {
      await logError('ApplicationService: Failed to signal project change', error);
      throw error;
    }
  }

  private handleInitializationStatus(status: InitializationStatus) {
    //console.log(`ApplicationService: Initialization status - ${status.message} (${status.progress}%)`);
    if (status.is_error) {
      console.error(`ApplicationService: Initialization error - ${status.error_details}`);
    }

    // Emit custom event for UI components to listen to
    window.dispatchEvent(
      new CustomEvent('initialization-status', {
        detail: status,
      })
    );
  }

  private handleInitializationComplete() {
    //console.log('ApplicationService: Backend initialization complete');

    // Emit custom event for UI components to listen to
    window.dispatchEvent(new CustomEvent('initialization-complete'));
  }

  private handleProjectChangeStatus(status: ProjectChangeStatus) {
    //console.log(`ApplicationService: Project change status - ${status.message} (${status.progress}%)`);
    if (status.is_error) {
      console.error(`ApplicationService: Project change error - ${status.error_details}`);
    }

    // Emit custom event for UI components to listen to
    window.dispatchEvent(
      new CustomEvent('project-change-status', {
        detail: status,
      })
    );
  }

  private handleProjectChangeComplete(complete: ProjectChangeComplete) {
    //console.log(`ApplicationService: Project change complete - ${complete.project_path}`);

    // Emit custom event for UI components to listen to
    window.dispatchEvent(
      new CustomEvent('project-change-complete', {
        detail: complete,
      })
    );
  }

  private handleCompute42Ready() {
    //console.log('ApplicationService: Compute42 is ready');

    // Emit custom event for UI components to listen to
    window.dispatchEvent(new CustomEvent('compute42-ready'));
  }

  /**
   * Clean up event listeners
   */
  async cleanup(): Promise<void> {
    try {
      // Clean up unified event listeners
      await unifiedEventService.removeAllListeners();

      // Clean up legacy event listeners
      if (this.initializationStatusUnlistenFn) {
        this.initializationStatusUnlistenFn();
        this.initializationStatusUnlistenFn = null;
      }
      if (this.initializationCompleteUnlistenFn) {
        this.initializationCompleteUnlistenFn();
        this.initializationCompleteUnlistenFn = null;
      }
      if (this.projectChangeStatusUnlistenFn) {
        this.projectChangeStatusUnlistenFn();
        this.projectChangeStatusUnlistenFn = null;
      }
      if (this.projectChangeCompleteUnlistenFn) {
        this.projectChangeCompleteUnlistenFn();
        this.projectChangeCompleteUnlistenFn = null;
      }
      if (this.compute42ReadyUnlistenFn) {
        this.compute42ReadyUnlistenFn();
        this.compute42ReadyUnlistenFn = null;
      }
      await trace('ApplicationService: Event listeners cleaned up successfully');
    } catch (error) {
      await logError('ApplicationService: Failed to cleanup event listeners', error);
    }
  }
}

// Export singleton instance
export const applicationService = new ApplicationService();
