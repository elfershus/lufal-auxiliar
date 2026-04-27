class NavState {
	open = $state(false);
	toggle() {
		this.open = !this.open;
	}
	close() {
		this.open = false;
	}
}

export const nav = new NavState();
