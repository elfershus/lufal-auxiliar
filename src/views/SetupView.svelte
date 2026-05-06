<script lang="ts">
    import { saveConfig } from "../lib/grpc.js";

    interface Props {
        onDone: () => void;
    }
    let { onDone }: Props = $props();

    let endpoint = $state("https://central.pinturaslufal.com:443");
    let apiKey = $state("");
    let showKey = $state(false);
    let saving = $state(false);
    let errorMsg = $state("");

    async function handleSubmit(e: Event) {
        e.preventDefault();
        if (!endpoint.trim() || !apiKey.trim()) return;

        saving = true;
        errorMsg = "";
        try {
            await saveConfig(endpoint.trim(), apiKey.trim());
            onDone();
        } catch (err) {
            errorMsg = err instanceof Error ? err.message : String(err);
        } finally {
            saving = false;
        }
    }
</script>

<div class="min-h-screen bg-bg flex items-center justify-center p-4">
    <div class="w-full max-w-sm">
        <!-- Logo / título -->
        <div class="text-center mb-8">
            <div
                class="w-16 h-16 rounded-2xl bg-navy flex items-center justify-center mx-auto mb-4"
            >
                <svg
                    class="w-8 h-8 text-white"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.75"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                >
                    <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
                </svg>
            </div>
            <h1
                class="font-barlow-condensed text-[28px] font-bold text-navy leading-none"
            >
                Lufal Auxiliar
            </h1>
            <p class="text-[13px] text-slate-400 mt-1">Configuración inicial</p>
        </div>

        <!-- Card -->
        <div class="bg-surface rounded-card p-6 shadow-card">
            <p class="text-[12px] text-slate-500 mb-5">
                Ingresa los datos de conexión para continuar. Sólo se requiere
                configurar una vez.
            </p>

            <form onsubmit={handleSubmit} class="flex flex-col gap-4">
                <!-- Endpoint -->
                <div>
                    <label
                        for="setup-endpoint"
                        class="block text-[11px] font-semibold uppercase tracking-[0.1em] text-slate-400 mb-1.5"
                    >
                        Servidor gRPC
                    </label>
                    <input
                        id="setup-endpoint"
                        type="text"
                        bind:value={endpoint}
                        placeholder="https://servidor:443"
                        autocomplete="off"
                        spellcheck="false"
                        class="w-full h-10 px-3 rounded-lg text-[13px] font-mono
							bg-bg border border-slate-200
							focus:outline-none focus:ring-2 focus:ring-amber/60
							text-slate-700 placeholder:text-slate-400"
                    />
                </div>

                <!-- API Key -->
                <div>
                    <label
                        for="setup-apikey"
                        class="block text-[11px] font-semibold uppercase tracking-[0.1em] text-slate-400 mb-1.5"
                    >
                        API Key
                    </label>
                    <div class="relative">
                        {#if showKey}
                            <input
                                id="setup-apikey"
                                type="text"
                                bind:value={apiKey}
                                placeholder="Clave de acceso"
                                autocomplete="off"
                                spellcheck="false"
                                class="w-full h-10 pl-3 pr-10 rounded-lg text-[13px] font-mono
									bg-bg border border-slate-200
									focus:outline-none focus:ring-2 focus:ring-amber/60
									text-slate-700 placeholder:text-slate-400"
                            />
                        {:else}
                            <input
                                id="setup-apikey"
                                type="password"
                                bind:value={apiKey}
                                placeholder="Clave de acceso"
                                autocomplete="off"
                                class="w-full h-10 pl-3 pr-10 rounded-lg text-[13px] font-mono
									bg-bg border border-slate-200
									focus:outline-none focus:ring-2 focus:ring-amber/60
									text-slate-700 placeholder:text-slate-400"
                            />
                        {/if}
                        <button
                            type="button"
                            onclick={() => (showKey = !showKey)}
                            class="absolute right-2 top-1/2 -translate-y-1/2 w-7 h-7 flex items-center justify-center
								text-slate-400 hover:text-slate-600 transition-colors"
                            tabindex="-1"
                        >
                            {#if showKey}
                                <svg
                                    class="w-4 h-4"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2"
                                    stroke-linecap="round"
                                >
                                    <path
                                        d="M17.94 17.94A10.07 10.07 0 0112 20c-7 0-11-8-11-8a18.45 18.45 0 015.06-5.94M9.9 4.24A9.12 9.12 0 0112 4c7 0 11 8 11 8a18.5 18.5 0 01-2.16 3.19m-6.72-1.07a3 3 0 11-4.24-4.24"
                                    />
                                    <line x1="1" y1="1" x2="23" y2="23" />
                                </svg>
                            {:else}
                                <svg
                                    class="w-4 h-4"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2"
                                    stroke-linecap="round"
                                >
                                    <path
                                        d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"
                                    />
                                    <circle cx="12" cy="12" r="3" />
                                </svg>
                            {/if}
                        </button>
                    </div>
                </div>

                {#if errorMsg}
                    <p class="text-[12px] text-red-500 -mt-1">{errorMsg}</p>
                {/if}

                <button
                    type="submit"
                    disabled={saving || !endpoint.trim() || !apiKey.trim()}
                    class="w-full h-10 rounded-lg text-[14px] font-medium font-barlow mt-1
						bg-amber text-white transition-opacity
						hover:opacity-90 active:opacity-80
						disabled:opacity-40 disabled:cursor-not-allowed"
                >
                    {saving ? "Conectando…" : "Guardar y continuar"}
                </button>
            </form>
        </div>
    </div>
</div>
