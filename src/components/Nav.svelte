<script lang="ts">
    import { nav } from "../lib/nav.svelte.js";
    import { appConfig } from "../lib/config.svelte.js";
    import { getVersion } from "@tauri-apps/api/app";

    interface Props {
        activeView: string;
        onNavigate: (view: string) => void;
    }
    let { activeView, onNavigate }: Props = $props();

    let appVersion = $state("...");
    $effect(() => {
        getVersion().then((v) => (appVersion = v));
    });

    function isActive(view: string): boolean {
        return activeView === view;
    }

    $effect(() => {
        void activeView;
        nav.close();
    });
</script>

{#if nav.open}
    <div
        class="md:hidden fixed inset-0 z-40 bg-black/60 backdrop-blur-[2px]"
        role="presentation"
        onclick={() => nav.close()}
    ></div>
{/if}

<aside
    class="fixed inset-y-0 left-0 z-50 w-52 flex flex-col
		bg-[#0f1f38] shadow-[2px_0_16px_rgba(0,0,0,0.3)]
		transition-transform duration-300 ease-in-out
		md:translate-x-0
		{nav.open ? 'translate-x-0' : '-translate-x-full'}"
    aria-label="Navegación principal"
>
    <!-- Encabezado -->
    <div
        class="flex items-start justify-between px-4 pt-5 pb-4 border-b border-white/10"
    >
        <div>
            <span
                class="block text-[10px] font-semibold tracking-[0.14em] uppercase text-white/40 mb-1"
            >
                Pinturas Lufal
            </span>
            <span
                class="block font-barlow-condensed text-[22px] font-bold text-white leading-none"
            >
                Menú
            </span>
        </div>
        <button
            class="md:hidden w-8 h-8 flex items-center justify-center rounded-lg text-white/40
				active:bg-white/10 transition-colors"
            onclick={() => nav.close()}
            aria-label="Cerrar menú"
        >
            <svg
                class="w-4 h-4"
                viewBox="0 0 16 16"
                fill="none"
                stroke="currentColor"
                stroke-width="1.75"
                stroke-linecap="round"
            >
                <path d="M3 3L13 13M13 3L3 13" />
            </svg>
        </button>
    </div>

    <!-- Links -->
    <nav class="flex-1 py-2 overflow-y-auto">
        <button
            class="w-full flex items-center gap-3 px-4 py-[10px] font-barlow text-[14px] font-medium
				no-underline transition-colors duration-150 border-l-2 text-left
				{isActive('ordenes')
                ? 'bg-white/[0.12] text-white border-l-[#e11d48]'
                : 'text-white/55 border-l-transparent hover:text-white hover:bg-white/[0.07]'}"
            onclick={() => onNavigate("ordenes")}
        >
            <span
                class="w-[18px] h-[18px] shrink-0 flex"
                style="color: {isActive('ordenes')
                    ? '#e11d48'
                    : 'rgba(255,255,255,0.35)'}"
            >
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.75"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                >
                    <path
                        d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"
                    />
                    <polyline points="14 2 14 8 20 8" />
                    <line x1="8" y1="13" x2="16" y2="13" />
                    <line x1="8" y1="17" x2="16" y2="17" />
                </svg>
            </span>
            <span>Órdenes de Compra</span>
        </button>

        <button
            class="w-full flex items-center gap-3 px-4 py-[10px] font-barlow text-[14px] font-medium
				no-underline transition-colors duration-150 border-l-2 text-left
				{isActive('fracciones')
                ? 'bg-white/[0.12] text-white border-l-[#e11d48]'
                : 'text-white/55 border-l-transparent hover:text-white hover:bg-white/[0.07]'}"
            onclick={() => onNavigate("fracciones")}
        >
            <span
                class="w-[18px] h-[18px] shrink-0 flex"
                style="color: {isActive('fracciones')
                    ? '#e11d48'
                    : 'rgba(255,255,255,0.35)'}"
            >
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.75"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                >
                    <line x1="19" y1="5" x2="5" y2="19" />
                    <circle cx="6.5" cy="6.5" r="2.5" />
                    <circle cx="17.5" cy="17.5" r="2.5" />
                </svg>
            </span>
            <span>Verificador de Fracciones</span>
        </button>
    </nav>

    <!-- Almacén activo -->
    <div class="mx-3 mb-2">
        <div
            class="rounded-lg bg-white/[0.06] border border-white/[0.08] px-3 py-2.5"
        >
            <div class="flex items-center gap-2">
                <div class="flex-1 min-w-0">
                    <div class="flex items-center gap-1 mb-1.5">
                        <svg
                            class="w-2.5 h-2.5 text-white/30 shrink-0"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        >
                            <path
                                d="M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2z"
                            />
                            <polyline points="9 22 9 12 15 12 15 22" />
                        </svg>
                        <p
                            class="text-[9px] font-semibold tracking-[0.14em] uppercase text-white/35"
                        >
                            Almacén activo
                        </p>
                    </div>
                    {#if appConfig.numalm}
                        {#if appConfig.nomalm}
                            <p
                                class="font-barlow-condensed text-[16px] font-bold text-white leading-snug truncate"
                            >
                                {appConfig.nomalm}
                            </p>
                        {/if}
                    {:else}
                        <div class="flex items-center gap-1.5">
                            <svg
                                class="w-3.5 h-3.5 text-amber/60 shrink-0"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <path
                                    d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"
                                />
                                <line x1="12" y1="9" x2="12" y2="13" />
                                <line x1="12" y1="17" x2="12.01" y2="17" />
                            </svg>
                            <span class="text-[11px] text-amber/60"
                                >Sin almacén configurado</span
                            >
                        </div>
                    {/if}
                </div>
                {#if appConfig.numalm}
                    <span
                        class="font-mono text-[11px] font-bold text-amber/80 bg-amber/10 border border-amber/20 px-1.5 py-1 rounded-md shrink-0 leading-none"
                    >
                        #{appConfig.numalm}
                    </span>
                {/if}
            </div>
        </div>
    </div>

    <!-- Configuración -->
    <div class="border-t border-white/[0.07] py-2">
        <button
            class="w-full flex items-center gap-3 px-4 py-[10px] font-barlow text-[14px] font-medium
				no-underline transition-colors duration-150 border-l-2 text-left
				{isActive('config')
                ? 'bg-white/[0.12] text-white border-l-amber'
                : 'text-white/55 border-l-transparent hover:text-white hover:bg-white/[0.07]'}"
            onclick={() => onNavigate("config")}
        >
            <span
                class="w-[18px] h-[18px] shrink-0 flex"
                style="color: {isActive('config')
                    ? '#e8820a'
                    : 'rgba(255,255,255,0.35)'}"
            >
                <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.75"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                >
                    <circle cx="12" cy="12" r="3" />
                    <path
                        d="M19.07 4.93a10 10 0 010 14.14M4.93 4.93a10 10 0 000 14.14"
                    />
                    <path
                        d="M12 2v2M12 20v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M2 12h2M20 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"
                    />
                </svg>
            </span>
            <span>Configuración</span>
        </button>
    </div>

    <!-- Pie -->
    <div class="px-4 py-3 border-t border-white/[0.07]">
        <p class="text-[10px] text-white/25 font-mono tracking-[0.04em]">
            v{appVersion} · Lufal Auxiliar
        </p>
    </div>
</aside>
