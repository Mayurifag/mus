# Makefile for Overmind process manager
# Ensure Overmind is installed: https://github.com/DarthSim/overmind

# Default Procfile name, can be overridden if needed
PROCFILE ?= Procfile

.PHONY: om-help
om-help:
	@echo "Overmind Makefile help:"
	@echo "  make om-up             Start processes defined in $(PROCFILE) (foreground)"
	@echo "  make om-up-d           Start processes defined in $(PROCFILE) (daemonized)"
	@echo "  make om-stop-all       Stop all processes managed by Overmind for this Procfile"
	@echo "  make om-restart-all    Restart all processes"
	@echo "  make om-restart-web    Restart the 'web' process (backend)"
	@echo "  make om-restart-front  Restart the 'frontend' process"
	@echo "  make om-connect-web    Connect to 'web' process output (backend)"
	@echo "  make om-connect-front  Connect to 'frontend' process output"
	@echo "  make om-status         Show status of Overmind processes"
	@echo "  make om-quit           Quit the Overmind daemon (stops all managed processes)"
	@echo ""
	@echo "Prerequisites:"
	@echo "  - Overmind must be installed and in your PATH."
	@echo "  - A $(PROCFILE) must exist in the project root."

.PHONY: om-up
om-up:
	@echo "Starting services with Overmind (foreground)... Access backend at http://localhost:8000, frontend at http://localhost:5173 (usually)"
	overmind start -f $(PROCFILE)

.PHONY: om-up-d
om-up-d:
	@echo "Starting services with Overmind (daemonized)..."
	overmind start -f $(PROCFILE) --daemonize
	@echo "Services started in background. Use 'make om-status' to check and 'make om-connect-web' or 'make om-connect-front' to view logs."

.PHONY: om-stop-all
om-stop-all:
	@echo "Stopping all Overmind managed processes for $(PROCFILE)..."
	overmind stop --all

.PHONY: om-restart-all
om-restart-all:
	@echo "Restarting all Overmind managed processes for $(PROCFILE)..."
	overmind restart --all

.PHONY: om-restart-web
om-restart-web:
	@echo "Restarting 'web' (backend) process..."
	overmind restart web

.PHONY: om-restart-front
om-restart-front:
	@echo "Restarting 'frontend' process..."
	overmind restart frontend

.PHONY: om-connect-web
om-connect-web:
	@echo "Connecting to 'web' (backend) process output. Press Ctrl+C to detach."
	overmind connect web

.PHONY: om-connect-front
om-connect-front:
	@echo "Connecting to 'frontend' process output. Press Ctrl+C to detach."
	overmind connect frontend

.PHONY: om-status
om-status:
	@echo "Overmind process status:"
	overmind status

.PHONY: om-quit
om-quit:
	@echo "Quitting Overmind daemon (will stop all managed processes)..."
	overmind quit
	@echo "Overmind daemon quit."
