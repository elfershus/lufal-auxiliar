<script lang="ts">
    import { onMount } from "svelte";
    import SkeletonList from "../components/SkeletonList.svelte";
    import ErrorBanner from "../components/ErrorBanner.svelte";
    import { nav } from "../lib/nav.svelte.js";
    import {
        getFraccionesInitData,
        getDbfPaths,
        saveFraccionPairing,
        deleteFraccionPairing,
        createEtiqueta,
        updateEtiqueta,
        deleteEtiqueta,
        setEmparejamientoEtiquetas,
        downloadPairingsTemplate,
        exportPairingsXlsx,
        parsePairingsXlsx,
        importPairings,
        addSeguimientoFraccion,
        deleteSeguimientoFraccion,
        downloadSeguimientosTemplate,
        parseSeguimientosXlsx,
        importSeguimientos,
    } from "../lib/dbf.js";
    import type {
        ArticuloFracciones,
        ArticuloSearchResult,
        ArticuloPareado,
        FraccionRecord,
        Etiqueta,
        PairingRow,
        ParsePairingsResult,
        SeguimientoFraccionRow,
        ParseSeguimientosResult,
        VerifNivel,
        VerifFraccion,
    } from "../lib/types.js";

    interface Props {
        onGoConfig: () => void;
    }
    let { onGoConfig }: Props = $props();

    // ── Paleta de colores para etiquetas ──────────────────────────
    const ETIQUETA_COLORS = [
        { name: "Azul", hex: "#3b82f6" },
        { name: "Violeta", hex: "#8b5cf6" },
        { name: "Rosa", hex: "#ec4899" },
        { name: "Rojo", hex: "#ef4444" },
        { name: "Naranja", hex: "#f97316" },
        { name: "Ámbar", hex: "#f59e0b" },
        { name: "Verde", hex: "#22c55e" },
        { name: "Teal", hex: "#14b8a6" },
    ];

    function etiquetaBadgeStyle(color: string): string {
        return `color:${color}; background:${color}1a; border-color:${color}33;`;
    }

    // ── Datos ──────────────────────────────────────────────────────
    let articulos = $state<ArticuloFracciones[]>([]);
    let todosArticulos = $state<ArticuloSearchResult[]>([]);
    let etiquetas = $state<Etiqueta[]>([]);
    let seguimientos = $state<SeguimientoFraccionRow[]>([]);
    let query = $state("");
    let cargando = $state(false);
    let errorMsg = $state("");
    let sinCarpeta = $state(false);
    let expandedKeys = $state(new Set<string>());

    // ── Estado etiquetas ───────────────────────────────────────────
    let filtroEtiquetaId = $state<number | null>(null);

    // Popover de asignación
    let popoverEmp = $state<FilaFraccion | null>(null);
    let popoverX = $state(0);
    let popoverY = $state(0);

    // Modal de gestión
    let modalEtiquetasOpen = $state(false);
    let editingEtiqueta = $state<Etiqueta | null>(null);
    let formNombre = $state("");
    let formColor = $state("#3b82f6");
    let formError = $state("");
    let formSaving = $state(false);

    type Emparejamiento = {
        numart_origen: string;
        desc_origen: string;
        unidad_base: string;
        frac: FraccionRecord;
        pareado: ArticuloPareado;
    };

    type FilaFraccion = {
        numart_origen: string;
        desc_origen: string;
        unidad_base: string;
        frac: FraccionRecord;
        verif: VerifFraccion;
    };

    let soloConProblemas = $state(false);

    const todosEmp = $derived<Emparejamiento[]>(
        articulos.flatMap((art) =>
            art.fracciones
                .filter((f) => f.pareado !== null)
                .map((f) => ({
                    numart_origen: art.numart,
                    desc_origen: art.desc,
                    unidad_base: art.unidad_base,
                    frac: f,
                    pareado: f.pareado!,
                })),
        ),
    );

    // ── Fuente unificada de todas las fracciones ──────────────────

    const seguimientosSet = $derived(
        new Set(
            seguimientos.map((s) => `${s.numart_origen}|${s.unidad_fraccion}`),
        ),
    );

    const todasFilas = $derived<FilaFraccion[]>(
        articulos.flatMap((art) =>
            art.fracciones
                .filter((frac) =>
                    seguimientosSet.has(`${art.numart}|${frac.unidad}`),
                )
                .map((frac) => ({
                    numart_origen: art.numart,
                    desc_origen: art.desc,
                    unidad_base: art.unidad_base,
                    frac,
                    verif: calcularVerifFraccion(art, frac),
                })),
        ),
    );

    const totalConProblemas = $derived(
        todasFilas.filter((f) => f.verif.hayProblema).length,
    );

    const filasFiltradas = $derived(
        todasFilas
            .filter((f) => !soloConProblemas || f.verif.hayProblema)
            .filter(
                (f) =>
                    filtroEtiquetaId === null ||
                    f.frac.etiquetas.some((etq) => etq.id === filtroEtiquetaId),
            )
            .filter(
                (f) =>
                    !query.trim() ||
                    matchTokens(
                        query,
                        f.desc_origen,
                        f.numart_origen,
                        f.frac.unidad,
                        ...(f.frac.pareado
                            ? [f.frac.pareado.desc, f.frac.pareado.numart]
                            : []),
                        ...f.frac.etiquetas.map((etq) => etq.nombre),
                    ),
            ),
    );

    // ── Verificación interna de precios entre fracciones ──────────

    type FuentePrecio = {
        unidad: string;
        factor: number;
        precios: number[];
    };

    type InconsistenciaDetalle = {
        fuenteA: string;
        fuenteB: string;
        nivel: number;
        efectivoA: number;
        efectivoB: number;
    };

    type ResultadoVerificacion = {
        art: ArticuloFracciones;
        fuentes: FuentePrecio[];
        inconsistencias: InconsistenciaDetalle[];
    };

    function getFactor(f: FraccionRecord): number {
        return f.equiv1 > 0 ? f.equiv2 / f.equiv1 : 1.0;
    }

    function calcularVerifFraccion(
        art: ArticuloFracciones,
        frac: FraccionRecord,
    ): VerifFraccion {
        const factor = getFactor(frac);
        const basePrecios = getPrecios5(art);
        const fracPrecios = getPrecios5(frac);
        const niveles: VerifNivel[] = [0, 1, 2, 3, 4].map((i) => {
            const precioActual = fracPrecios[i];
            const precioMinimo =
                frac.equiv1 > 0
                    ? (basePrecios[i] * frac.equiv2) / frac.equiv1
                    : basePrecios[i];
            const desactualizado =
                basePrecios[i] > 0 && precioActual < precioMinimo;
            return {
                nivel: i + 1,
                precioActual,
                precioMinimo,
                diferencia: precioActual - precioMinimo,
                desactualizado,
            };
        });
        const nivelesConProblema = niveles
            .filter((v) => v.desactualizado)
            .map((v) => v.nivel);
        return {
            factor,
            niveles,
            hayProblema: nivelesConProblema.length > 0,
            nivelesConProblema,
        };
    }

    function efectivoPorBase(precio: number, factor: number): number {
        return factor > 0 ? precio / factor : 0;
    }

    function getPrecios5(obj: {
        precio1: number;
        precio2: number;
        precio3: number;
        precio4: number;
        precio5: number;
    }): number[] {
        return [
            obj.precio1,
            obj.precio2,
            obj.precio3,
            obj.precio4,
            obj.precio5,
        ];
    }

    function verificarArticulo(art: ArticuloFracciones): ResultadoVerificacion {
        const fuentes: FuentePrecio[] = [
            { unidad: art.unidad_base, factor: 1.0, precios: getPrecios5(art) },
            ...art.fracciones
                .filter((f) => f.pareado !== null)
                .map((f) => ({
                    unidad: f.unidad,
                    factor: getFactor(f),
                    precios: getPrecios5(f),
                })),
        ];

        const inconsistencias: InconsistenciaDetalle[] = [];
        for (let i = 0; i < fuentes.length; i++) {
            for (let j = i + 1; j < fuentes.length; j++) {
                const [fA, fB] =
                    fuentes[i].factor <= fuentes[j].factor
                        ? [fuentes[i], fuentes[j]]
                        : [fuentes[j], fuentes[i]];
                for (let n = 0; n < 5; n++) {
                    const pA = fA.precios[n],
                        pB = fB.precios[n];
                    if (pA <= 0 || pB <= 0) continue;
                    const eA = efectivoPorBase(pA, fA.factor);
                    const eB = efectivoPorBase(pB, fB.factor);
                    if (eA < eB * 0.999) {
                        inconsistencias.push({
                            fuenteA: fA.unidad,
                            fuenteB: fB.unidad,
                            nivel: n + 1,
                            efectivoA: eA,
                            efectivoB: eB,
                        });
                    }
                }
            }
        }
        return { art, fuentes, inconsistencias };
    }

    const resultadosVerificacion = $derived<ResultadoVerificacion[]>(
        articulos
            .filter((a) => a.fracciones.some((f) => f.pareado !== null))
            .map(verificarArticulo),
    );

    const conteoConInconsistencias = $derived(
        resultadosVerificacion.filter((r) => r.inconsistencias.length > 0)
            .length,
    );

    function getVerifFraccion(
        numart_origen: string,
        unidad_fraccion: string,
    ): {
        fuente_base: FuentePrecio;
        fuente_frac: FuentePrecio;
        incons: InconsistenciaDetalle[];
    } | null {
        const resultado = resultadosVerificacion.find(
            (r) => r.art.numart === numart_origen,
        );
        if (!resultado) return null;
        const fuente_frac = resultado.fuentes.find(
            (f) => f.unidad === unidad_fraccion,
        );
        const fuente_base = resultado.fuentes.find((f) => f.factor === 1.0);
        if (!fuente_frac || !fuente_base) return null;
        return {
            fuente_base,
            fuente_frac,
            incons: resultado.inconsistencias.filter(
                (inc) =>
                    inc.fuenteA === unidad_fraccion ||
                    inc.fuenteB === unidad_fraccion,
            ),
        };
    }

    // ── Modal de nuevo emparejamiento (3 pasos) ───────────────────
    // Paso 0: cerrado
    // Paso 1: buscar artículo origen (client-side en `articulos`)
    // Paso 2: elegir fracción del artículo origen
    // Paso 3: buscar artículo destino (client-side en `todosArticulos`)
    let paso = $state<0 | 1 | 2 | 3>(0);
    let queryOrigen = $state("");
    let artOrigenActivo = $state<ArticuloFracciones | null>(null);
    let fraccionActiva = $state<{ numart: string; unidad: string } | null>(
        null,
    );
    let queryDestino = $state("");

    const resultadosOrigen = $derived(
        queryOrigen.trim()
            ? articulos
                  .filter((a) => matchTokens(queryOrigen, a.desc, a.numart))
                  .slice(0, 30)
            : [],
    );

    const resultadosDestino = $derived(
        queryDestino.trim()
            ? todosArticulos
                  .filter((a) => matchTokens(queryDestino, a.desc, a.numart))
                  .slice(0, 30)
            : [],
    );

    // ── Carga ──────────────────────────────────────────────────────
    onMount(cargar);

    async function cargar() {
        cargando = true;
        errorMsg = "";
        sinCarpeta = false;
        articulos = [];
        todosArticulos = [];
        try {
            const paths = await getDbfPaths();
            if (!paths.dbf_arts || !paths.dbf_unidades) {
                sinCarpeta = true;
                return;
            }
            const data = await getFraccionesInitData();
            articulos = data.fracciones;
            todosArticulos = data.articulos;
            etiquetas = data.etiquetas;
            seguimientos = data.seguimientos;
        } catch (e) {
            errorMsg = e instanceof Error ? e.message : String(e);
        } finally {
            cargando = false;
        }
    }

    // ── Búsqueda por tokens ───────────────────────────────────────
    function matchTokens(q: string, ...fields: string[]): boolean {
        const tokens = q.trim().toLowerCase().split(/\s+/).filter(Boolean);
        if (tokens.length === 0) return true;
        const haystack = fields.join(" ").toLowerCase();
        return tokens.every((t) => haystack.includes(t));
    }

    // ── Helpers visuales ──────────────────────────────────────────
    function fmt(v: number): string {
        return v.toLocaleString("es-MX", {
            minimumFractionDigits: 2,
            maximumFractionDigits: 2,
        });
    }

    type ConPrecios = {
        precio1: number;
        precio2: number;
        precio3: number;
        precio4: number;
        precio5: number;
    };

    function getPrecio(obj: ConPrecios, n: number): number {
        const map: Record<number, number> = {
            1: obj.precio1,
            2: obj.precio2,
            3: obj.precio3,
            4: obj.precio4,
            5: obj.precio5,
        };
        return map[n] ?? 0;
    }

    function pctDif(
        frac: number,
        par: number,
    ): { texto: string; clase: string } {
        if (Math.abs(frac) < 0.001)
            return { texto: "—", clase: "text-slate-300" };
        const d = par - frac;
        if (Math.abs(d / frac) < 0.01)
            return { texto: "=", clase: "text-slate-400" };
        const pct = (d / frac) * 100;
        return {
            texto: `${pct > 0 ? "+" : ""}${pct.toFixed(1)}%`,
            clase: pct > 0 ? "text-red-500" : "text-emerald-600",
        };
    }

    function difFmt(
        frac: number,
        par: number,
    ): { texto: string; clase: string } {
        const d = par - frac;
        if (Math.abs(d) < 0.001) return { texto: "=", clase: "text-slate-400" };
        if (frac !== 0 && Math.abs(d / frac) < 0.01)
            return { texto: "=", clase: "text-slate-400" };
        const pct = frac !== 0 ? ((d / frac) * 100).toFixed(1) : "—";
        const signo = d > 0 ? "+" : "";
        return {
            texto: `${signo}$${fmt(d)} (${signo}${pct}%)`,
            clase: d > 0 ? "text-red-500" : "text-emerald-600",
        };
    }

    // ── Acciones de seguimiento ───────────────────────────────────
    async function quitarSeguimiento(
        numart_origen: string,
        unidad_fraccion: string,
    ) {
        try {
            await deleteSeguimientoFraccion(numart_origen, unidad_fraccion);
            seguimientos = seguimientos.filter(
                (s) =>
                    !(
                        s.numart_origen === numart_origen &&
                        s.unidad_fraccion === unidad_fraccion
                    ),
            );
        } catch (e) {
            errorMsg = e instanceof Error ? e.message : String(e);
        }
    }

    // ── Modal de nuevo seguimiento (2 pasos) ──────────────────────
    let pasoSeg = $state<0 | 1 | 2>(0);
    let queryOrigenSeg = $state("");
    let artOrigenActivoSeg = $state<ArticuloFracciones | null>(null);

    const resultadosOrigenSeg = $derived(
        queryOrigenSeg.trim()
            ? articulos
                  .filter((a) => matchTokens(queryOrigenSeg, a.desc, a.numart))
                  .slice(0, 30)
            : [],
    );

    function abrirModalSeg() {
        pasoSeg = 1;
        queryOrigenSeg = "";
        artOrigenActivoSeg = null;
    }

    function cerrarModalSeg() {
        pasoSeg = 0;
        queryOrigenSeg = "";
        artOrigenActivoSeg = null;
    }

    function seleccionarOrigenSeg(art: ArticuloFracciones) {
        artOrigenActivoSeg = art;
        pasoSeg = 2;
    }

    function onQueryOrigenSegInput(e: Event) {
        queryOrigenSeg = (e.target as HTMLInputElement).value.toUpperCase();
        (e.target as HTMLInputElement).value = queryOrigenSeg;
    }

    async function seleccionarFraccionSeg(numart: string, unidad: string) {
        try {
            await addSeguimientoFraccion(numart, unidad);
            seguimientos = [
                ...seguimientos,
                { numart_origen: numart, unidad_fraccion: unidad },
            ];
            cerrarModalSeg();
        } catch (e) {
            errorMsg = e instanceof Error ? e.message : String(e);
        }
    }

    // ── XLSX de seguimientos ──────────────────────────────────────
    let xlsxSegModalOpen = $state(false);
    let xlsxSegParsing = $state(false);
    let xlsxSegPreview = $state<ParseSeguimientosResult | null>(null);
    let xlsxSegMode = $state<"agregar" | "reemplazar">("agregar");
    let xlsxSegImporting = $state(false);
    let xlsxSegResult = $state<number | null>(null);
    let xlsxSegError = $state("");

    async function abrirImportXlsxSeg() {
        xlsxSegError = "";
        xlsxSegPreview = null;
        xlsxSegResult = null;
        xlsxSegParsing = true;
        try {
            const result = await parseSeguimientosXlsx();
            if (result) {
                xlsxSegPreview = result;
                xlsxSegModalOpen = true;
            }
        } catch (e) {
            xlsxSegError = e instanceof Error ? e.message : String(e);
            xlsxSegModalOpen = true;
        } finally {
            xlsxSegParsing = false;
        }
    }

    function cerrarModalXlsxSeg() {
        xlsxSegModalOpen = false;
        xlsxSegPreview = null;
        xlsxSegResult = null;
        xlsxSegError = "";
        xlsxSegMode = "agregar";
    }

    async function confirmarImportSeg() {
        if (!xlsxSegPreview) return;
        const validRows: SeguimientoFraccionRow[] = xlsxSegPreview.rows
            .filter((r) => r.errors.length === 0)
            .map((r) => ({
                numart_origen: r.numart_origen,
                unidad_fraccion: r.unidad_fraccion,
            }));
        if (validRows.length === 0) return;
        xlsxSegImporting = true;
        xlsxSegError = "";
        try {
            const n = await importSeguimientos(validRows, xlsxSegMode);
            xlsxSegResult = n;
            await cargar();
        } catch (e) {
            xlsxSegError = e instanceof Error ? e.message : String(e);
        } finally {
            xlsxSegImporting = false;
        }
    }

    // ── Acciones ──────────────────────────────────────────────────
    async function desvincular(numart_origen: string, unidad_fraccion: string) {
        try {
            await deleteFraccionPairing(numart_origen, unidad_fraccion);
            await cargar();
        } catch (e) {
            errorMsg = e instanceof Error ? e.message : String(e);
        }
    }

    function toggleExpand(key: string) {
        const next = new Set(expandedKeys);
        if (next.has(key)) next.delete(key);
        else next.add(key);
        expandedKeys = next;
    }

    function focusOnMount(node: HTMLElement) {
        node.focus();
    }

    // ── Popover de etiquetas ──────────────────────────────────────
    function abrirPopover(f: FilaFraccion, anchor: HTMLElement) {
        const rect = anchor.getBoundingClientRect();
        popoverX = Math.min(rect.left, window.innerWidth - 272);
        popoverY = rect.bottom + 6;
        popoverEmp = f;
    }

    function cerrarPopover() {
        popoverEmp = null;
    }

    function clickFuera(node: HTMLElement, handler: () => void) {
        const listener = (ev: MouseEvent) => {
            if (!node.contains(ev.target as Node)) handler();
        };
        document.addEventListener("mousedown", listener, true);
        return {
            destroy() {
                document.removeEventListener("mousedown", listener, true);
            },
        };
    }

    async function toggleEtiqueta(fila: FilaFraccion, etiquetaId: number) {
        const ya = fila.frac.etiquetas.some((e) => e.id === etiquetaId);
        const nuevosIds = ya
            ? fila.frac.etiquetas
                  .filter((e) => e.id !== etiquetaId)
                  .map((e) => e.id)
            : [...fila.frac.etiquetas.map((e) => e.id), etiquetaId];

        // Actualización optimista
        const art = articulos.find((a) => a.numart === fila.numart_origen);
        if (art) {
            const frac = art.fracciones.find(
                (f) => f.unidad === fila.frac.unidad,
            );
            if (frac) {
                frac.etiquetas = ya
                    ? frac.etiquetas.filter((e) => e.id !== etiquetaId)
                    : [
                          ...frac.etiquetas,
                          etiquetas.find((e) => e.id === etiquetaId)!,
                      ];
            }
        }
        articulos = [...articulos];

        try {
            await setEmparejamientoEtiquetas(
                fila.numart_origen,
                fila.frac.unidad,
                nuevosIds,
            );
        } catch (err) {
            errorMsg = err instanceof Error ? err.message : String(err);
            await cargar();
        }
    }

    // ── Modal gestión de etiquetas ────────────────────────────────
    function abrirModalEtiquetas() {
        modalEtiquetasOpen = true;
        editingEtiqueta = null;
        formNombre = "";
        formColor = "#3b82f6";
        formError = "";
    }

    function cerrarModalEtiquetas() {
        modalEtiquetasOpen = false;
        editingEtiqueta = null;
        formError = "";
    }

    function empezarEditar(etq: Etiqueta) {
        editingEtiqueta = etq;
        formNombre = etq.nombre;
        formColor = etq.color;
        formError = "";
    }

    function cancelarEdicion() {
        editingEtiqueta = null;
        formNombre = "";
        formColor = "#3b82f6";
        formError = "";
    }

    async function guardarEtiqueta() {
        if (!formNombre.trim()) return;
        formSaving = true;
        formError = "";
        try {
            if (editingEtiqueta) {
                await updateEtiqueta(
                    editingEtiqueta.id,
                    formNombre.trim(),
                    formColor,
                );
                const id = editingEtiqueta.id;
                etiquetas = etiquetas.map((e) =>
                    e.id === id
                        ? { ...e, nombre: formNombre.trim(), color: formColor }
                        : e,
                );
                articulos = articulos.map((art) => ({
                    ...art,
                    fracciones: art.fracciones.map((frac) => ({
                        ...frac,
                        etiquetas: frac.etiquetas.map((etq) =>
                            etq.id === id
                                ? {
                                      ...etq,
                                      nombre: formNombre.trim(),
                                      color: formColor,
                                  }
                                : etq,
                        ),
                    })),
                }));
                editingEtiqueta = null;
            } else {
                const nueva = await createEtiqueta(
                    formNombre.trim(),
                    formColor,
                );
                etiquetas = [...etiquetas, nueva];
            }
            formNombre = "";
            formColor = "#3b82f6";
        } catch (err) {
            formError = err instanceof Error ? err.message : String(err);
        } finally {
            formSaving = false;
        }
    }

    async function eliminarEtiqueta(id: number) {
        try {
            await deleteEtiqueta(id);
            etiquetas = etiquetas.filter((e) => e.id !== id);
            if (filtroEtiquetaId === id) filtroEtiquetaId = null;
            await cargar();
        } catch (err) {
            errorMsg = err instanceof Error ? err.message : String(err);
        }
    }

    function filtrarPorEtiqueta(id: number) {
        filtroEtiquetaId = filtroEtiquetaId === id ? null : id;
    }

    function handleKeyEtiqueta(ev: KeyboardEvent) {
        if (ev.key === "Enter") guardarEtiqueta();
    }

    function handleKeyModalEtiquetas(ev: KeyboardEvent) {
        if (ev.key === "Escape") cerrarModalEtiquetas();
    }

    function handleKeyPopover(ev: KeyboardEvent) {
        if (ev.key === "Escape") cerrarPopover();
    }

    // ── Modal de nuevo emparejamiento ─────────────────────────────
    function vincularDirecto(f: FilaFraccion) {
        const art = articulos.find((a) => a.numart === f.numart_origen);
        if (!art) return;
        artOrigenActivo = art;
        fraccionActiva = { numart: f.numart_origen, unidad: f.frac.unidad };
        queryDestino = "";
        paso = 3;
    }

    function abrirModal() {
        paso = 1;
        queryOrigen = "";
        artOrigenActivo = null;
        fraccionActiva = null;
        queryDestino = "";
    }

    function cerrarModal() {
        paso = 0;
        queryOrigen = "";
        artOrigenActivo = null;
        fraccionActiva = null;
        queryDestino = "";
    }

    function seleccionarOrigen(art: ArticuloFracciones) {
        artOrigenActivo = art;
        paso = 2;
    }

    function seleccionarFraccion(numart: string, unidad: string) {
        fraccionActiva = { numart, unidad };
        queryDestino = "";
        paso = 3;
    }

    function onQueryOrigenInput(e: Event) {
        queryOrigen = (e.target as HTMLInputElement).value.toUpperCase();
        (e.target as HTMLInputElement).value = queryOrigen;
    }

    function onQueryDestinoInput(e: Event) {
        const input = e.target as HTMLInputElement;
        const upper = input.value.toUpperCase();
        input.value = upper;
        queryDestino = upper;
    }

    async function seleccionarDestino(destino: ArticuloSearchResult) {
        if (!fraccionActiva) return;
        try {
            await saveFraccionPairing(
                fraccionActiva.numart,
                fraccionActiva.unidad,
                destino.numart,
            );
            cerrarModal();
            await cargar();
        } catch (e) {
            errorMsg = e instanceof Error ? e.message : String(e);
        }
    }

    // ── XLSX import/export ────────────────────────────────────────
    let xlsxModalOpen = $state(false);
    let xlsxParsing = $state(false);
    let xlsxPreview = $state<ParsePairingsResult | null>(null);
    let xlsxMode = $state<"agregar" | "reemplazar">("agregar");
    let xlsxImporting = $state(false);
    let xlsxResult = $state<number | null>(null);
    let xlsxError = $state("");
    let xlsxExporting = $state(false);

    async function descargarPlantilla() {
        try {
            await downloadPairingsTemplate();
        } catch (e) {
            errorMsg = e instanceof Error ? e.message : String(e);
        }
    }

    async function descargarPlantillaSeg() {
        try {
            await downloadSeguimientosTemplate();
        } catch (e) {
            errorMsg = e instanceof Error ? e.message : String(e);
        }
    }

    async function exportarXlsx() {
        xlsxExporting = true;
        try {
            await exportPairingsXlsx();
        } catch (e) {
            errorMsg = e instanceof Error ? e.message : String(e);
        } finally {
            xlsxExporting = false;
        }
    }

    async function abrirImportXlsx() {
        xlsxError = "";
        xlsxPreview = null;
        xlsxResult = null;
        xlsxParsing = true;
        try {
            const result = await parsePairingsXlsx();
            if (result) {
                xlsxPreview = result;
                xlsxModalOpen = true;
            }
        } catch (e) {
            xlsxError = e instanceof Error ? e.message : String(e);
            xlsxModalOpen = true;
        } finally {
            xlsxParsing = false;
        }
    }

    function cerrarModalXlsx() {
        xlsxModalOpen = false;
        xlsxPreview = null;
        xlsxResult = null;
        xlsxError = "";
        xlsxMode = "agregar";
    }

    async function confirmarImport() {
        if (!xlsxPreview) return;
        const validRows: PairingRow[] = xlsxPreview.rows
            .filter((r) => r.errors.length === 0)
            .map((r) => ({
                numart_origen: r.numart_origen,
                unidad_fraccion: r.unidad_fraccion,
                numart_destino: r.numart_destino,
            }));
        if (validRows.length === 0) return;
        xlsxImporting = true;
        xlsxError = "";
        try {
            const n = await importPairings(validRows, xlsxMode);
            xlsxResult = n;
            await cargar();
        } catch (e) {
            xlsxError = e instanceof Error ? e.message : String(e);
        } finally {
            xlsxImporting = false;
        }
    }
</script>

<div class="min-h-screen bg-bg">
    <!-- ── Header ──────────────────────────────────────────────── -->
    <div class="bg-[#0f1f38] px-4 pt-5 pb-4 md:px-6">
        <div class="flex items-center justify-between">
            <div class="flex items-center gap-3">
                <button
                    class="md:hidden w-9 h-9 flex items-center justify-center rounded-lg text-white/60
						hover:bg-white/10 active:bg-white/20 transition-colors"
                    onclick={() => nav.toggle()}
                    aria-label="Abrir menú"
                >
                    <svg
                        class="w-5 h-5"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="1.75"
                        stroke-linecap="round"
                    >
                        <line x1="3" y1="6" x2="21" y2="6" />
                        <line x1="3" y1="12" x2="21" y2="12" />
                        <line x1="3" y1="18" x2="21" y2="18" />
                    </svg>
                </button>
                <div>
                    <p
                        class="text-[11px] font-semibold tracking-[0.12em] uppercase text-white/40"
                    >
                        Inventario
                    </p>
                    <h1
                        class="font-barlow-condensed text-[22px] font-bold text-white leading-none"
                    >
                        Verificador de Fracciones
                    </h1>
                </div>
            </div>

            <div class="flex items-center gap-3">
                <!-- Grupo Seguimientos -->
                {#if !sinCarpeta && !cargando}
                    <div
                        class="flex items-center gap-1.5 px-2 py-1 rounded-xl bg-white/[0.06] border border-white/10"
                    >
                        <span
                            class="text-[9px] font-semibold tracking-[0.12em] uppercase text-white/30 pr-0.5 select-none"
                            >Seg.</span
                        >
                        <!-- Gestionar etiquetas -->
                        <button
                            onclick={abrirModalEtiquetas}
                            class="h-8 px-3 flex items-center gap-1.5 rounded-lg bg-white/10 border border-white/15 text-white text-[12px] font-medium hover:bg-white/20 active:bg-white/25 transition-colors"
                        >
                            <svg
                                class="w-3.5 h-3.5"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <path
                                    d="M20.59 13.41l-7.17 7.17a2 2 0 01-2.83 0L2 12V2h10l8.59 8.59a2 2 0 010 2.82z"
                                />
                                <line x1="7" y1="7" x2="7.01" y2="7" />
                            </svg>
                            Etiquetas
                        </button>
                        <!-- Seguimientos: Plantilla -->
                        <button
                            onclick={descargarPlantillaSeg}
                            class="h-8 px-3 flex items-center gap-1.5 rounded-lg bg-white/10 border border-white/15 text-white text-[12px] font-medium hover:bg-white/20 active:bg-white/25 transition-colors"
                            title="Descargar plantilla XLSX de seguimientos"
                        >
                            <svg
                                class="w-3.5 h-3.5"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <path
                                    d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"
                                />
                                <polyline points="17 8 12 3 7 8" />
                                <line x1="12" y1="3" x2="12" y2="15" />
                            </svg>
                            Plantilla
                        </button>
                        <!-- Seguimientos: Importar -->
                        <button
                            onclick={abrirImportXlsxSeg}
                            disabled={xlsxSegParsing}
                            class="h-8 px-3 flex items-center gap-1.5 rounded-lg bg-white/10 border border-white/15 text-white text-[12px] font-medium hover:bg-white/20 active:bg-white/25 disabled:opacity-40 transition-colors"
                            title="Importar seguimientos desde XLSX"
                        >
                            <svg
                                class="w-3.5 h-3.5"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <path
                                    d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"
                                />
                                <polyline points="7 10 12 15 17 10" />
                                <line x1="12" y1="15" x2="12" y2="3" />
                            </svg>
                            {xlsxSegParsing ? "Leyendo…" : "Importar"}
                        </button>
                        <!-- Seguimientos: Agregar -->
                        <button
                            onclick={abrirModalSeg}
                            class="h-8 px-3 flex items-center gap-1.5 rounded-lg bg-amber text-white text-[12px] font-medium hover:opacity-90 active:opacity-80 transition-opacity"
                            title="Agregar fracción al seguimiento"
                        >
                            <svg
                                class="w-3.5 h-3.5"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                            >
                                <line x1="12" y1="5" x2="12" y2="19" /><line
                                    x1="5"
                                    y1="12"
                                    x2="19"
                                    y2="12"
                                />
                            </svg>
                            Seguimiento
                        </button>
                    </div>
                {/if}
                <!-- Grupo Emparejamientos -->
                {#if !sinCarpeta && !cargando}
                    <div
                        class="flex items-center gap-1.5 px-2 py-1 rounded-xl bg-white/[0.06] border border-white/10"
                    >
                        <span
                            class="text-[9px] font-semibold tracking-[0.12em] uppercase text-white/30 pr-0.5 select-none"
                            >Emp.</span
                        >
                        <!-- Plantilla XLSX -->
                        <button
                            onclick={descargarPlantilla}
                            class="h-8 px-3 flex items-center gap-1.5 rounded-lg bg-white/10 border border-white/15 text-white text-[12px] font-medium hover:bg-white/20 active:bg-white/25 transition-colors"
                            title="Descargar plantilla XLSX vacía"
                        >
                            <svg
                                class="w-3.5 h-3.5"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <path
                                    d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"
                                />
                                <polyline points="17 8 12 3 7 8" />
                                <line x1="12" y1="3" x2="12" y2="15" />
                            </svg>
                            Plantilla
                        </button>
                        <!-- Exportar XLSX -->
                        {#if todosEmp.length > 0}
                            <button
                                onclick={exportarXlsx}
                                disabled={xlsxExporting}
                                class="h-8 px-3 flex items-center gap-1.5 rounded-lg bg-white/10 border border-white/15 text-white text-[12px] font-medium hover:bg-white/20 active:bg-white/25 disabled:opacity-40 transition-colors"
                                title="Exportar emparejamientos a XLSX"
                            >
                                <svg
                                    class="w-3.5 h-3.5"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                >
                                    <path
                                        d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"
                                    />
                                    <polyline points="14 2 14 8 20 8" />
                                    <line x1="16" y1="13" x2="8" y2="13" />
                                    <line x1="16" y1="17" x2="8" y2="17" />
                                    <polyline points="10 9 9 9 8 9" />
                                </svg>
                                Exportar
                            </button>
                        {/if}
                        <!-- Importar XLSX -->
                        <button
                            onclick={abrirImportXlsx}
                            disabled={xlsxParsing}
                            class="h-8 px-3 flex items-center gap-1.5 rounded-lg bg-white/10 border border-white/15 text-white text-[12px] font-medium hover:bg-white/20 active:bg-white/25 disabled:opacity-40 transition-colors"
                            title="Importar emparejamientos desde XLSX"
                        >
                            <svg
                                class="w-3.5 h-3.5"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <path
                                    d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"
                                />
                                <polyline points="7 10 12 15 17 10" />
                                <line x1="12" y1="15" x2="12" y2="3" />
                            </svg>
                            {xlsxParsing ? "Leyendo…" : "Importar"}
                        </button>
                        <!-- Nuevo emparejamiento -->
                        <button
                            onclick={abrirModal}
                            class="h-8 px-3 flex items-center gap-1.5 rounded-lg bg-amber text-white text-[12px] font-medium hover:opacity-90 active:opacity-80 transition-opacity"
                        >
                            <svg
                                class="w-3.5 h-3.5"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                            >
                                <line x1="12" y1="5" x2="12" y2="19" /><line
                                    x1="5"
                                    y1="12"
                                    x2="19"
                                    y2="12"
                                />
                            </svg>
                            Nuevo
                        </button>
                    </div>
                {/if}
                <!-- Recargar -->
                <button
                    onclick={cargar}
                    disabled={cargando}
                    class="w-9 h-9 flex items-center justify-center rounded-lg text-white/60 hover:bg-white/10 active:bg-white/20 transition-colors disabled:opacity-40"
                    title="Releer archivos DBF"
                >
                    <svg
                        class="w-4 h-4 {cargando ? 'animate-spin-fast' : ''}"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="1.75"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <polyline points="23 4 23 10 17 10" />
                        <path d="M20.49 15a9 9 0 11-2.12-9.36L23 10" />
                    </svg>
                </button>
            </div>
        </div>

        <!-- Barra de búsqueda -->
        {#if !sinCarpeta && !cargando && todasFilas.length > 0}
            <div class="mt-4 relative">
                <svg
                    class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-white/30 pointer-events-none"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.75"
                    stroke-linecap="round"
                >
                    <circle cx="11" cy="11" r="8" /><line
                        x1="21"
                        y1="21"
                        x2="16.65"
                        y2="16.65"
                    />
                </svg>
                <input
                    type="text"
                    placeholder="Buscar por artículo, fracción, clave o etiqueta…"
                    bind:value={query}
                    class="w-full h-9 pl-9 pr-3 rounded-lg text-[13px] font-barlow
						bg-white/[0.08] border border-white/10 text-white placeholder:text-white/30
						focus:outline-none focus:ring-1 focus:ring-white/20 focus:bg-white/[0.12] transition-colors"
                />
            </div>
            <!-- Pastillas de filtro -->
            <div class="mt-2 flex items-center gap-1.5 flex-wrap">
                <span
                    class="text-[10px] font-semibold uppercase tracking-wider text-white/30 mr-0.5"
                    >Filtrar:</span
                >
                <button
                    onclick={() => {
                        soloConProblemas = !soloConProblemas;
                        filtroEtiquetaId = null;
                    }}
                    class="h-6 px-2.5 rounded-full text-[11px] font-medium border transition-colors flex items-center gap-1
						{soloConProblemas
                        ? 'bg-red-500/20 text-red-300 border-red-400/40 font-semibold'
                        : 'text-white/50 border-white/20 hover:bg-white/10'}"
                >
                    <svg
                        class="w-3 h-3 flex-shrink-0"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2.5"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <path
                            d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"
                        />
                        <line x1="12" y1="9" x2="12" y2="13" /><line
                            x1="12"
                            y1="17"
                            x2="12.01"
                            y2="17"
                        />
                    </svg>
                    Solo con problemas
                    {#if totalConProblemas > 0}
                        <span
                            class="ml-0.5 bg-red-500 text-white text-[9px] font-bold rounded-full px-1.5 leading-4"
                            >{totalConProblemas}</span
                        >
                    {/if}
                </button>
                {#if etiquetas.length > 0}
                    <span class="text-white/20 text-[11px]">|</span>
                    <button
                        onclick={() => {
                            filtroEtiquetaId = null;
                            soloConProblemas = false;
                        }}
                        class="h-6 px-2.5 rounded-full text-[11px] font-medium transition-colors
							{filtroEtiquetaId === null && !soloConProblemas
                            ? 'bg-white/20 text-white'
                            : 'text-white/50 hover:bg-white/10'}">Todos</button
                    >
                    {#each etiquetas as etq (etq.id)}
                        <button
                            onclick={() => {
                                filtrarPorEtiqueta(etq.id);
                                soloConProblemas = false;
                            }}
                            style={filtroEtiquetaId === etq.id
                                ? `background:${etq.color}33; color:${etq.color}; border-color:${etq.color}66;`
                                : ""}
                            class="h-6 px-2.5 rounded-full text-[11px] font-medium border transition-colors
								{filtroEtiquetaId === etq.id
                                ? 'border'
                                : 'text-white/50 border-white/20 hover:bg-white/10'}"
                            >{etq.nombre}</button
                        >
                    {/each}
                {/if}
            </div>
        {/if}
    </div>

    <!-- ── Contenido ───────────────────────────────────────────── -->
    <div class="px-4 py-4 md:px-6">
        {#if errorMsg}
            <ErrorBanner message={errorMsg} />
        {/if}

        {#if sinCarpeta}
            <div
                class="mt-8 flex flex-col items-center text-center animate-fadeSlide"
            >
                <div
                    class="w-14 h-14 rounded-full bg-amber/10 flex items-center justify-center mb-4"
                >
                    <svg
                        class="w-7 h-7 text-amber"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="1.75"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <path
                            d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"
                        />
                    </svg>
                </div>
                <h2
                    class="font-barlow-condensed text-[18px] font-bold text-slate-700 mb-1"
                >
                    Archivos DBF no configurados
                </h2>
                <p class="text-[13px] text-slate-400 mb-5 max-w-xs">
                    Configura el archivo de artículos y el de fracciones en
                    Configuración.
                </p>
                <button
                    onclick={onGoConfig}
                    class="h-9 px-5 rounded-lg bg-navy text-white text-[13px] font-medium font-barlow
						hover:opacity-90 active:opacity-80 transition-opacity"
                >
                    Ir a Configuración
                </button>
            </div>
        {:else if cargando}
            <SkeletonList count={4} />
        {:else}
            <!-- ── Fracciones ─────────────────────────────────────────────── -->
            {#if articulos.length === 0}
                <!-- Sin fracciones en DBF -->
                <div
                    class="mt-12 flex flex-col items-center text-center animate-fadeSlide"
                >
                    <div
                        class="w-14 h-14 rounded-full bg-navy/8 flex items-center justify-center mb-4"
                    >
                        <svg
                            class="w-7 h-7 text-navy/40"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.5"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        >
                            <rect
                                x="2"
                                y="3"
                                width="20"
                                height="14"
                                rx="2"
                            /><line x1="8" y1="21" x2="16" y2="21" /><line
                                x1="12"
                                y1="17"
                                x2="12"
                                y2="21"
                            />
                        </svg>
                    </div>
                    <h2
                        class="font-barlow-condensed text-[18px] font-bold text-slate-600 mb-1"
                    >
                        Sin fracciones
                    </h2>
                    <p class="text-[13px] text-slate-400 max-w-xs">
                        No se encontraron fracciones en los archivos DBF.
                    </p>
                </div>
            {:else if seguimientos.length === 0}
                <!-- Hay fracciones pero no hay seguimientos configurados -->
                <div
                    class="mt-12 flex flex-col items-center text-center animate-fadeSlide"
                >
                    <div
                        class="w-14 h-14 rounded-full bg-sky-50 flex items-center justify-center mb-4"
                    >
                        <svg
                            class="w-7 h-7 text-sky-400"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.5"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        >
                            <circle cx="11" cy="11" r="8" />
                            <line x1="21" y1="21" x2="16.65" y2="16.65" />
                        </svg>
                    </div>
                    <h2
                        class="font-barlow-condensed text-[18px] font-bold text-slate-600 mb-1"
                    >
                        Sin seguimientos
                    </h2>
                    <p class="text-[13px] text-slate-400 mb-5 max-w-xs">
                        Agrega fracciones al seguimiento para comenzar la
                        verificación. Hay {articulos.reduce(
                            (acc, a) => acc + a.fracciones.length,
                            0,
                        )} fraccion{articulos.reduce(
                            (a, art) => a + art.fracciones.length,
                            0,
                        ) !== 1
                            ? "es"
                            : ""} disponibles en el DBF.
                    </p>
                    <button
                        onclick={abrirModalSeg}
                        class="h-9 px-5 rounded-lg bg-navy text-white text-[13px] font-medium font-barlow
                                hover:opacity-90 active:opacity-80 transition-opacity"
                    >
                        + Agregar seguimiento
                    </button>
                </div>
            {:else if filasFiltradas.length === 0 && soloConProblemas}
                <div
                    class="mt-12 flex flex-col items-center text-center animate-fadeSlide"
                >
                    <div
                        class="w-14 h-14 rounded-full bg-emerald-50 flex items-center justify-center mb-4"
                    >
                        <svg
                            class="w-7 h-7 text-emerald-500"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        >
                            <polyline points="20 6 9 17 4 12" />
                        </svg>
                    </div>
                    <h2
                        class="font-barlow-condensed text-[18px] font-bold text-slate-600 mb-1"
                    >
                        Todo actualizado
                    </h2>
                    <p class="text-[13px] text-slate-400 max-w-xs">
                        Todas las fracciones tienen precios por encima del
                        mínimo esperado.
                    </p>
                </div>
            {:else if filasFiltradas.length === 0}
                <div class="mt-10 text-center animate-fadeSlide">
                    <p class="text-[14px] text-slate-400">
                        Sin resultados para "{query}"
                    </p>
                </div>
            {:else}
                <div class="flex items-center justify-between mb-3">
                    <p
                        class="text-[11px] text-slate-400 font-medium tracking-wide"
                    >
                        {filasFiltradas.length} fracción{filasFiltradas.length !==
                        1
                            ? "es"
                            : ""}
                        {query.trim() ||
                        soloConProblemas ||
                        filtroEtiquetaId !== null
                            ? `· filtrado de ${todasFilas.length}`
                            : ""}
                    </p>
                    <div class="flex items-center gap-2">
                        {#if totalConProblemas > 0}
                            <span
                                class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-semibold bg-red-50 text-red-600 border border-red-200"
                            >
                                <svg
                                    class="w-3 h-3 flex-shrink-0"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2.5"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                >
                                    <path
                                        d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"
                                    />
                                    <line x1="12" y1="9" x2="12" y2="13" /><line
                                        x1="12"
                                        y1="17"
                                        x2="12.01"
                                        y2="17"
                                    />
                                </svg>
                                {totalConProblemas} desactualizadas
                            </span>
                        {/if}
                        {#if conteoConInconsistencias > 0}
                            <span
                                class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-semibold bg-amber/10 text-amber border border-amber/30"
                            >
                                <svg
                                    class="w-3 h-3 flex-shrink-0"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2.5"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                >
                                    <path
                                        d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"
                                    />
                                    <line x1="12" y1="9" x2="12" y2="13" /><line
                                        x1="12"
                                        y1="17"
                                        x2="12.01"
                                        y2="17"
                                    />
                                </svg>
                                {conteoConInconsistencias} inc. de pareo
                            </span>
                        {/if}
                    </div>
                </div>

                <div
                    class="bg-surface rounded-card shadow-card overflow-hidden"
                >
                    <table class="w-full border-collapse">
                        <thead>
                            <tr class="bg-bg/70 border-b border-slate-100">
                                <th
                                    class="text-left px-4 py-2.5 text-[10px] font-semibold uppercase tracking-wider text-slate-400 w-[22%]"
                                    >Clave origen</th
                                >
                                <th
                                    class="text-center px-2 py-2.5 text-[10px] font-semibold uppercase tracking-wider text-slate-400 w-[6%]"
                                    >Base</th
                                >
                                <th
                                    class="text-center px-2 py-2.5 text-[10px] font-semibold uppercase tracking-wider text-slate-400 w-[7%]"
                                    >Fracc.</th
                                >
                                <th
                                    class="text-center px-2 py-2.5 text-[10px] font-semibold uppercase tracking-wider text-slate-400 w-[8%]"
                                    >Factor</th
                                >
                                <th
                                    class="text-left px-4 py-2.5 text-[10px] font-semibold uppercase tracking-wider text-slate-400 w-[18%]"
                                    >Emparejamiento</th
                                >
                                <th
                                    class="text-left px-3 py-2.5 text-[10px] font-semibold uppercase tracking-wider text-slate-400 w-[18%]"
                                    >Etiquetas</th
                                >
                                <th
                                    class="hidden sm:table-cell text-center px-3 py-2.5 text-[10px] font-semibold uppercase tracking-wider text-slate-400 w-[13%]"
                                    >Niveles</th
                                >
                                <th class="w-[8%]"></th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each filasFiltradas as f (`${f.numart_origen}|${f.frac.unidad}`)}
                                {@const key = `${f.numart_origen}|${f.frac.unidad}`}
                                {@const isOpen = expandedKeys.has(key)}
                                {@const verifFrac = getVerifFraccion(
                                    f.numart_origen,
                                    f.frac.unidad,
                                )}
                                {@const tieneInconsPairing =
                                    verifFrac !== null &&
                                    verifFrac.incons.length > 0}
                                {@const tienePareado = f.frac.pareado !== null}

                                <!-- Fila resumen -->
                                <tr
                                    class="border-t border-slate-100 hover:bg-bg/50 transition-colors cursor-pointer group"
                                    onclick={() => toggleExpand(key)}
                                >
                                    <td class="px-4 py-2.5 min-w-0">
                                        <p
                                            class="font-mono text-[13px] font-bold text-navy leading-snug truncate"
                                        >
                                            {f.numart_origen}
                                        </p>
                                        <p
                                            class="text-[11px] text-slate-400 leading-none mt-0.5 truncate"
                                        >
                                            {f.desc_origen}
                                        </p>
                                    </td>
                                    <td class="px-2 py-2.5 text-center">
                                        <span
                                            class="inline-block text-[11px] font-semibold text-slate-600 bg-slate-100 border border-slate-200 px-1.5 py-0.5 rounded font-mono leading-none whitespace-nowrap"
                                            >{f.unidad_base}</span
                                        >
                                    </td>
                                    <td class="px-2 py-2.5 text-center">
                                        <span
                                            class="inline-block text-[11px] font-semibold px-1.5 py-0.5 rounded font-mono leading-none whitespace-nowrap
											{f.verif.hayProblema
                                                ? 'text-red-600 bg-red-50 border border-red-300'
                                                : tieneInconsPairing
                                                  ? 'text-amber bg-amber/10 border border-amber/30'
                                                  : 'text-slate-600 bg-slate-100 border border-slate-200'}"
                                        >
                                            {f.frac.unidad}
                                        </span>
                                        {#if f.verif.hayProblema}
                                            <p
                                                class="text-[9px] text-red-500 font-bold mt-0.5 whitespace-nowrap"
                                            >
                                                ↓ {f.verif.nivelesConProblema
                                                    .length}
                                            </p>
                                        {/if}
                                    </td>
                                    <td class="px-2 py-2.5 text-center">
                                        <span
                                            class="font-mono text-[11px] text-slate-500 whitespace-nowrap"
                                        >
                                            {f.frac.equiv1}/{f.frac.equiv2}
                                        </span>
                                        <p
                                            class="text-[9px] text-slate-400 mt-0.5"
                                        >
                                            {f.verif.factor.toFixed(3)}×
                                        </p>
                                    </td>
                                    <td class="px-4 py-2.5 min-w-0">
                                        {#if tienePareado}
                                            <p
                                                class="font-mono text-[12px] font-bold text-slate-600 leading-snug truncate"
                                            >
                                                {f.frac.pareado!.numart}
                                            </p>
                                            <p
                                                class="text-[11px] text-slate-400 leading-none mt-0.5 truncate"
                                            >
                                                {f.frac.pareado!.desc}
                                            </p>
                                        {:else}
                                            <span
                                                class="text-[11px] text-slate-300 italic"
                                                >Sin emparejamiento</span
                                            >
                                        {/if}
                                    </td>
                                    <!-- Columna Etiquetas -->
                                    <td class="px-3 py-2.5 w-[18%]">
                                        <div
                                            class="flex flex-wrap gap-1 items-center"
                                        >
                                            {#each f.frac.etiquetas as etq (etq.id)}
                                                <button
                                                    onclick={(ev) => {
                                                        ev.stopPropagation();
                                                        filtrarPorEtiqueta(
                                                            etq.id,
                                                        );
                                                    }}
                                                    style={etiquetaBadgeStyle(
                                                        etq.color,
                                                    )}
                                                    class="inline-block text-[11px] font-semibold px-1.5 py-0.5 rounded border font-mono leading-none whitespace-nowrap hover:opacity-70 transition-opacity"
                                                    >{etq.nombre}</button
                                                >
                                            {/each}
                                            {#if tienePareado}
                                                <button
                                                    onclick={(ev) => {
                                                        ev.stopPropagation();
                                                        abrirPopover(
                                                            f,
                                                            ev.currentTarget as HTMLElement,
                                                        );
                                                    }}
                                                    class="w-5 h-5 flex items-center justify-center rounded-full text-slate-400
														hover:bg-slate-100 hover:text-slate-600 transition-colors flex-shrink-0"
                                                    title="Asignar etiquetas"
                                                >
                                                    <svg
                                                        class="w-3 h-3"
                                                        viewBox="0 0 24 24"
                                                        fill="none"
                                                        stroke="currentColor"
                                                        stroke-width="2.5"
                                                        stroke-linecap="round"
                                                    >
                                                        <line
                                                            x1="12"
                                                            y1="5"
                                                            x2="12"
                                                            y2="19"
                                                        /><line
                                                            x1="5"
                                                            y1="12"
                                                            x2="19"
                                                            y2="12"
                                                        />
                                                    </svg>
                                                </button>
                                            {/if}
                                        </div>
                                    </td>
                                    <!-- Columna Niveles (indicadores de estado por precio) -->
                                    <td
                                        class="hidden sm:table-cell px-3 py-2.5"
                                    >
                                        <div
                                            class="flex flex-wrap gap-x-1 gap-y-0.5 justify-center"
                                        >
                                            {#each f.verif.niveles as nv}
                                                <span
                                                    class="font-mono text-[10px] font-bold px-1 py-0.5 rounded leading-none
													{nv.desactualizado
                                                        ? 'text-red-600 bg-red-50 border border-red-200'
                                                        : 'text-emerald-600 bg-emerald-50 border border-emerald-200'}"
                                                >
                                                    P{nv.nivel}
                                                </span>
                                            {/each}
                                        </div>
                                    </td>
                                    <td class="px-3 py-2.5 text-right">
                                        <div
                                            class="flex items-center justify-end gap-1"
                                        >
                                            <button
                                                onclick={(ev) => {
                                                    ev.stopPropagation();
                                                    quitarSeguimiento(
                                                        f.numart_origen,
                                                        f.frac.unidad,
                                                    );
                                                }}
                                                class="w-7 h-7 flex items-center justify-center rounded-md text-slate-300
                                                        hover:text-red-400 hover:bg-red-50 transition-colors"
                                                title="Quitar de seguimiento"
                                            >
                                                <svg
                                                    class="w-3.5 h-3.5"
                                                    viewBox="0 0 24 24"
                                                    fill="none"
                                                    stroke="currentColor"
                                                    stroke-width="2"
                                                    stroke-linecap="round"
                                                    stroke-linejoin="round"
                                                >
                                                    <polyline
                                                        points="3 6 5 6 21 6"
                                                    />
                                                    <path
                                                        d="M19 6l-1 14a2 2 0 01-2 2H8a2 2 0 01-2-2L5 6"
                                                    />
                                                    <path
                                                        d="M10 11v6M14 11v6"
                                                    />
                                                    <path
                                                        d="M9 6V4a1 1 0 011-1h4a1 1 0 011 1v2"
                                                    />
                                                </svg>
                                            </button>
                                            {#if tienePareado}
                                                <button
                                                    onclick={(ev) => {
                                                        ev.stopPropagation();
                                                        desvincular(
                                                            f.numart_origen,
                                                            f.frac.unidad,
                                                        );
                                                    }}
                                                    class="w-7 h-7 flex items-center justify-center rounded-md text-slate-400
														hover:text-red-500 hover:bg-red-50 transition-colors"
                                                    title="Desvincular"
                                                >
                                                    <svg
                                                        class="w-3.5 h-3.5"
                                                        viewBox="0 0 24 24"
                                                        fill="none"
                                                        stroke="currentColor"
                                                        stroke-width="2"
                                                        stroke-linecap="round"
                                                    >
                                                        <path
                                                            d="M18.36 6.64a9 9 0 11-12.73 0"
                                                        /><line
                                                            x1="12"
                                                            y1="2"
                                                            x2="12"
                                                            y2="12"
                                                        />
                                                    </svg>
                                                </button>
                                            {:else}
                                                <button
                                                    onclick={(ev) => {
                                                        ev.stopPropagation();
                                                        vincularDirecto(f);
                                                    }}
                                                    class="w-7 h-7 flex items-center justify-center rounded-md text-slate-400
														hover:text-navy hover:bg-sky-50 transition-colors"
                                                    title="Crear emparejamiento"
                                                >
                                                    <svg
                                                        class="w-3.5 h-3.5"
                                                        viewBox="0 0 24 24"
                                                        fill="none"
                                                        stroke="currentColor"
                                                        stroke-width="2"
                                                        stroke-linecap="round"
                                                        stroke-linejoin="round"
                                                    >
                                                        <path
                                                            d="M10 13a5 5 0 007.54.54l3-3a5 5 0 00-7.07-7.07l-1.72 1.71"
                                                        />
                                                        <path
                                                            d="M14 11a5 5 0 00-7.54-.54l-3 3a5 5 0 007.07 7.07l1.71-1.71"
                                                        />
                                                    </svg>
                                                </button>
                                            {/if}
                                            <div
                                                class="w-7 h-7 flex items-center justify-center rounded-md text-slate-400 group-hover:text-slate-600 transition-colors"
                                            >
                                                <svg
                                                    class="w-4 h-4 transition-transform duration-200 {isOpen
                                                        ? 'rotate-180'
                                                        : 'rotate-0'}"
                                                    viewBox="0 0 24 24"
                                                    fill="none"
                                                    stroke="currentColor"
                                                    stroke-width="2"
                                                    stroke-linecap="round"
                                                    stroke-linejoin="round"
                                                >
                                                    <polyline
                                                        points="6 9 12 15 18 9"
                                                    />
                                                </svg>
                                            </div>
                                        </div>
                                    </td>
                                </tr>

                                <!-- Fila detalle (expandida) -->
                                {#if isOpen}
                                    <tr class="border-t border-slate-100">
                                        <td
                                            colspan="8"
                                            class="px-0 py-0 bg-bg/40"
                                        >
                                            <table
                                                class="w-full text-[12px] mb-2"
                                            >
                                                <thead>
                                                    <tr class="bg-bg/60">
                                                        <th
                                                            class="text-left px-6 py-2 font-semibold text-slate-400 tracking-wide uppercase text-[10px] w-12"
                                                            >Nivel</th
                                                        >
                                                        <th
                                                            class="text-right px-4 py-2 font-semibold text-slate-400 tracking-wide uppercase text-[10px]"
                                                        >
                                                            Precio base <span
                                                                class="font-mono normal-case text-slate-500"
                                                                >({f.unidad_base})</span
                                                            >
                                                        </th>
                                                        <th
                                                            class="text-right px-4 py-2 font-semibold text-slate-400 tracking-wide uppercase text-[10px]"
                                                        >
                                                            Mínimo esperado <span
                                                                class="font-mono normal-case text-slate-500"
                                                                >(base×factor)</span
                                                            >
                                                        </th>
                                                        <th
                                                            class="text-right px-4 py-2 font-semibold text-slate-400 tracking-wide uppercase text-[10px]"
                                                        >
                                                            Precio fracción <span
                                                                class="font-mono normal-case text-slate-500"
                                                                >({f.frac
                                                                    .unidad})</span
                                                            >
                                                        </th>
                                                        <th
                                                            class="text-right px-4 py-2 font-semibold text-slate-400 tracking-wide uppercase text-[10px]"
                                                            >Diferencia</th
                                                        >
                                                        <th
                                                            class="text-center px-4 py-2 font-semibold text-slate-400 tracking-wide uppercase text-[10px]"
                                                            >Estado</th
                                                        >
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    {#each f.verif.niveles as nv}
                                                        {@const basePrecioNivel =
                                                            f.verif.factor > 0
                                                                ? nv.precioMinimo /
                                                                  f.verif.factor
                                                                : 0}
                                                        {@const difPct =
                                                            nv.precioMinimo > 0
                                                                ? (nv.diferencia /
                                                                      nv.precioMinimo) *
                                                                  100
                                                                : null}
                                                        <tr
                                                            class="border-t border-slate-100 hover:bg-bg/40 transition-colors {nv.desactualizado
                                                                ? 'bg-red-50/40'
                                                                : ''}"
                                                        >
                                                            <td
                                                                class="px-6 py-2.5 font-semibold text-slate-500 text-[13px]"
                                                                >P{nv.nivel}</td
                                                            >
                                                            <td
                                                                class="px-4 py-2.5 text-right font-mono text-[11px] text-slate-500"
                                                                >${fmt(
                                                                    basePrecioNivel,
                                                                )}</td
                                                            >
                                                            <td
                                                                class="px-4 py-2.5 text-right font-mono text-[11px] text-slate-500"
                                                                title="${fmt(
                                                                    basePrecioNivel,
                                                                )} × {f.verif.factor.toFixed(
                                                                    4,
                                                                )} = ${fmt(
                                                                    nv.precioMinimo,
                                                                )}"
                                                            >
                                                                ${fmt(
                                                                    nv.precioMinimo,
                                                                )}
                                                            </td>
                                                            <td
                                                                class="px-4 py-2.5 text-right font-mono text-[11px] text-slate-700"
                                                                >${fmt(
                                                                    nv.precioActual,
                                                                )}</td
                                                            >
                                                            <td
                                                                class="px-4 py-2.5 text-right font-semibold {nv.desactualizado
                                                                    ? 'text-red-600'
                                                                    : 'text-emerald-600'}"
                                                            >
                                                                {#if difPct !== null}
                                                                    <span
                                                                        class="font-mono text-[11px]"
                                                                    >
                                                                        {difPct >=
                                                                        0
                                                                            ? "+"
                                                                            : ""}{difPct.toFixed(
                                                                            1,
                                                                        )}%
                                                                    </span>
                                                                {:else}
                                                                    <span
                                                                        class="text-[10px] text-slate-300"
                                                                        >—</span
                                                                    >
                                                                {/if}
                                                            </td>
                                                            <td
                                                                class="px-4 py-2.5 text-center"
                                                            >
                                                                {#if nv.desactualizado}
                                                                    <span
                                                                        class="inline-flex items-center gap-1 text-[10px] font-bold text-red-600 bg-red-50 border border-red-200 px-1.5 py-0.5 rounded"
                                                                    >
                                                                        ↓ Bajo
                                                                        mínimo
                                                                    </span>
                                                                {:else if nv.precioMinimo > 0}
                                                                    <span
                                                                        class="inline-flex items-center gap-1 text-[10px] font-bold text-emerald-600 bg-emerald-50 border border-emerald-200 px-1.5 py-0.5 rounded"
                                                                    >
                                                                        ✓ OK
                                                                    </span>
                                                                {:else}
                                                                    <span
                                                                        class="text-[10px] text-slate-300"
                                                                        >—</span
                                                                    >
                                                                {/if}
                                                            </td>
                                                        </tr>
                                                    {/each}
                                                </tbody>
                                            </table>

                                            <!-- Sección B: Verificación vs. artículo pareado (solo si tiene pairing) -->
                                            {#if tienePareado}
                                                <div
                                                    class="px-4 pt-2 pb-1 border-t border-slate-100"
                                                >
                                                    <p
                                                        class="text-[10px] font-semibold uppercase tracking-wider text-slate-400 mb-1"
                                                    >
                                                        Verificación vs.
                                                        artículo emparejado
                                                        <span
                                                            class="font-mono normal-case text-slate-500 font-normal"
                                                        >
                                                            · {f.frac.pareado!
                                                                .numart} — {f
                                                                .frac.pareado!
                                                                .desc}
                                                        </span>
                                                    </p>
                                                </div>
                                                <table
                                                    class="w-full text-[12px] mb-2"
                                                >
                                                    <thead>
                                                        <tr class="bg-bg/60">
                                                            <th
                                                                class="text-left px-6 py-2 font-semibold text-slate-400 tracking-wide uppercase text-[10px] w-12"
                                                                >Nivel</th
                                                            >
                                                            <th
                                                                class="text-right px-4 py-2 font-semibold text-slate-400 tracking-wide uppercase text-[10px]"
                                                            >
                                                                Efect. base <span
                                                                    class="font-mono normal-case text-slate-500"
                                                                    >({f.unidad_base})</span
                                                                >
                                                            </th>
                                                            <th
                                                                class="text-right px-4 py-2 font-semibold text-slate-400 tracking-wide uppercase text-[10px]"
                                                            >
                                                                Efect. fracc. <span
                                                                    class="font-mono normal-case text-slate-500"
                                                                    >({f.frac
                                                                        .unidad})</span
                                                                >
                                                            </th>
                                                            <th
                                                                class="text-right px-4 py-2 font-semibold text-slate-400 tracking-wide uppercase text-[10px]"
                                                            >
                                                                Fracc. ÷ factor <span
                                                                    class="font-mono normal-case text-slate-500"
                                                                    >({f.frac
                                                                        .unidad})</span
                                                                >
                                                            </th>
                                                            <th
                                                                class="text-right px-4 py-2 font-semibold text-slate-400 tracking-wide uppercase text-[10px]"
                                                            >
                                                                Precio
                                                                emparejado <span
                                                                    class="font-mono normal-case text-slate-500"
                                                                    >({f.frac
                                                                        .pareado!
                                                                        .unidad})</span
                                                                >
                                                            </th>
                                                            <th
                                                                class="text-right px-4 py-2 font-semibold text-slate-400 tracking-wide uppercase text-[10px]"
                                                                >Dif. %</th
                                                            >
                                                        </tr>
                                                    </thead>
                                                    <tbody>
                                                        {#each [1, 2, 3, 4, 5] as n}
                                                            {@const pFrac =
                                                                getPrecio(
                                                                    f.frac,
                                                                    n,
                                                                )}
                                                            {@const pPar =
                                                                getPrecio(
                                                                    f.frac
                                                                        .pareado!,
                                                                    n,
                                                                )}
                                                            {@const dif =
                                                                difFmt(
                                                                    pFrac,
                                                                    pPar,
                                                                )}
                                                            {@const efectBase =
                                                                verifFrac
                                                                    ? verifFrac
                                                                          .fuente_base
                                                                          .precios[
                                                                          n - 1
                                                                      ]
                                                                    : 0}
                                                            {@const efectFrac =
                                                                verifFrac
                                                                    ? verifFrac
                                                                          .fuente_frac
                                                                          .precios[
                                                                          n - 1
                                                                      ]
                                                                    : 0}
                                                            {@const fracFactor =
                                                                verifFrac
                                                                    ? verifFrac
                                                                          .fuente_frac
                                                                          .precios[
                                                                          n - 1
                                                                      ] *
                                                                      verifFrac
                                                                          .fuente_frac
                                                                          .factor
                                                                    : 0}
                                                            {@const isInconsistent =
                                                                verifFrac !==
                                                                    null &&
                                                                verifFrac.incons.some(
                                                                    (inc) =>
                                                                        inc.nivel ===
                                                                        n,
                                                                )}
                                                            <tr
                                                                class="border-t border-slate-100 hover:bg-bg/40 transition-colors"
                                                            >
                                                                <td
                                                                    class="px-6 py-2.5 font-semibold text-slate-500 text-[13px]"
                                                                    >P{n}</td
                                                                >
                                                                <td
                                                                    class="px-4 py-2.5 text-right font-mono text-[11px] text-slate-500"
                                                                    >{verifFrac
                                                                        ? `$${fmt(efectBase)}`
                                                                        : "—"}</td
                                                                >
                                                                <td
                                                                    class="px-4 py-2.5 text-right font-mono text-[11px] text-slate-500"
                                                                    >{verifFrac
                                                                        ? `$${fmt(efectFrac)}`
                                                                        : "—"}</td
                                                                >
                                                                <td
                                                                    class="px-4 py-2.5 text-right font-mono text-[11px] {isInconsistent
                                                                        ? 'bg-red-50 text-red-700 font-bold ring-1 ring-inset ring-red-200'
                                                                        : 'text-slate-500'}"
                                                                    title={isInconsistent
                                                                        ? "Comprar por fracción es más barato que por unidad base"
                                                                        : undefined}
                                                                >
                                                                    {verifFrac
                                                                        ? `$${fmt(fracFactor)}`
                                                                        : "—"}
                                                                </td>
                                                                <td
                                                                    class="px-4 py-2.5 text-right font-mono text-[11px] text-slate-700"
                                                                    >${fmt(
                                                                        pPar,
                                                                    )}</td
                                                                >
                                                                <td
                                                                    class="px-4 py-2.5 text-right font-mono font-semibold {dif.clase}"
                                                                    >{dif.texto}</td
                                                                >
                                                            </tr>
                                                        {/each}
                                                    </tbody>
                                                </table>
                                            {/if}
                                        </td>
                                    </tr>
                                {/if}
                            {/each}
                        </tbody>
                    </table>
                </div>
            {/if}
        {/if}
    </div>
</div>

<!-- ── Popover de etiquetas ─────────────────────────────────────── -->
{#if popoverEmp !== null}
    {@const popEmp = popoverEmp}
    <div
        class="fixed z-40 bg-white rounded-xl shadow-2xl border border-slate-100 w-64 p-3"
        style="top:{popoverY}px; left:{popoverX}px;"
        use:clickFuera={cerrarPopover}
        onkeydown={handleKeyPopover}
        role="dialog"
        aria-modal="true"
        tabindex="-1"
    >
        <p
            class="text-[10px] font-semibold uppercase tracking-wider text-slate-400 mb-2"
        >
            Etiquetas · <span class="font-mono normal-case"
                >{popEmp.frac.unidad}</span
            >
        </p>
        {#if etiquetas.length === 0}
            <p class="text-[12px] text-slate-400 text-center py-2">
                Sin etiquetas creadas
            </p>
        {:else}
            <div class="flex flex-col gap-0.5">
                {#each etiquetas as etq (etq.id)}
                    {@const activa = popEmp.frac.etiquetas.some(
                        (e) => e.id === etq.id,
                    )}
                    <button
                        onclick={() => toggleEtiqueta(popEmp, etq.id)}
                        class="flex items-center gap-2 px-2 py-1.5 rounded-lg hover:bg-slate-50 transition-colors text-left"
                    >
                        <div
                            class="w-4 h-4 rounded border-2 flex items-center justify-center flex-shrink-0 transition-colors
							{activa ? 'border-navy bg-navy' : 'border-slate-300'}"
                        >
                            {#if activa}
                                <svg
                                    class="w-2.5 h-2.5 text-white"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="3"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                >
                                    <polyline points="20 6 9 17 4 12" />
                                </svg>
                            {/if}
                        </div>
                        <span
                            style={etiquetaBadgeStyle(etq.color)}
                            class="text-[11px] font-semibold px-1.5 py-0.5 rounded border font-mono leading-none"
                            >{etq.nombre}</span
                        >
                    </button>
                {/each}
            </div>
        {/if}
        <div class="mt-2 pt-2 border-t border-slate-100">
            <button
                onclick={() => {
                    cerrarPopover();
                    abrirModalEtiquetas();
                }}
                class="text-[11px] text-navy font-medium hover:underline"
                >+ Gestionar etiquetas</button
            >
        </div>
    </div>
{/if}

<!-- ── Modal de gestión de etiquetas ───────────────────────────── -->
{#if modalEtiquetasOpen}
    <div
        class="fixed inset-0 bg-black/40 z-50 flex items-end sm:items-center justify-center p-4"
        onclick={cerrarModalEtiquetas}
        onkeydown={handleKeyModalEtiquetas}
        role="dialog"
        aria-modal="true"
        tabindex="-1"
    >
        <div
            class="bg-white rounded-xl shadow-2xl w-full max-w-sm max-h-[80vh] flex flex-col overflow-hidden"
            onclick={(ev) => ev.stopPropagation()}
            onkeydown={(ev) => ev.stopPropagation()}
            role="document"
        >
            <!-- Header -->
            <div
                class="px-4 pt-4 pb-3 border-b border-slate-100 flex items-center justify-between"
            >
                <div>
                    <p
                        class="text-[11px] font-semibold uppercase tracking-wider text-slate-400"
                    >
                        Configuración
                    </p>
                    <h2
                        class="font-barlow-condensed text-[17px] font-bold text-navy"
                    >
                        Gestionar etiquetas
                    </h2>
                </div>
                <button
                    onclick={cerrarModalEtiquetas}
                    aria-label="Cerrar"
                    class="w-8 h-8 flex items-center justify-center rounded-lg text-slate-400
						hover:bg-slate-100 hover:text-slate-600 transition-colors"
                >
                    <svg
                        class="w-4 h-4"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                    >
                        <line x1="18" y1="6" x2="6" y2="18" /><line
                            x1="6"
                            y1="6"
                            x2="18"
                            y2="18"
                        />
                    </svg>
                </button>
            </div>

            <!-- Lista de etiquetas existentes -->
            <div class="overflow-y-auto flex-1 divide-y divide-slate-50">
                {#if etiquetas.length === 0}
                    <p class="text-center text-[13px] text-slate-400 py-8">
                        Sin etiquetas creadas aún
                    </p>
                {:else}
                    {#each etiquetas as etq (etq.id)}
                        <div class="px-4 py-2.5 flex items-center gap-3">
                            <span
                                style={etiquetaBadgeStyle(etq.color)}
                                class="text-[11px] font-semibold px-1.5 py-0.5 rounded border font-mono leading-none flex-shrink-0"
                                >{etq.nombre}</span
                            >
                            <span class="flex-1"></span>
                            <button
                                onclick={() => empezarEditar(etq)}
                                class="w-7 h-7 flex items-center justify-center rounded-md text-slate-400
									hover:text-navy hover:bg-slate-100 transition-colors"
                                title="Editar"
                            >
                                <svg
                                    class="w-3.5 h-3.5"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                >
                                    <path
                                        d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"
                                    />
                                    <path
                                        d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"
                                    />
                                </svg>
                            </button>
                            <button
                                onclick={() => eliminarEtiqueta(etq.id)}
                                class="w-7 h-7 flex items-center justify-center rounded-md text-slate-400
									hover:text-red-500 hover:bg-red-50 transition-colors"
                                title="Eliminar"
                            >
                                <svg
                                    class="w-3.5 h-3.5"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                >
                                    <polyline points="3 6 5 6 21 6" />
                                    <path
                                        d="M19 6l-1 14a2 2 0 01-2 2H8a2 2 0 01-2-2L5 6"
                                    />
                                    <path d="M10 11v6M14 11v6" />
                                    <path
                                        d="M9 6V4a1 1 0 011-1h4a1 1 0 011 1v2"
                                    />
                                </svg>
                            </button>
                        </div>
                    {/each}
                {/if}
            </div>

            <!-- Formulario crear / editar -->
            <div class="px-4 py-3 border-t border-slate-100">
                <div class="flex items-center justify-between mb-2">
                    <p
                        class="text-[11px] font-semibold uppercase tracking-wider text-slate-400"
                    >
                        {editingEtiqueta ? "Editar etiqueta" : "Nueva etiqueta"}
                    </p>
                    {#if editingEtiqueta}
                        <button
                            onclick={cancelarEdicion}
                            class="text-[11px] text-slate-400 hover:text-slate-600 transition-colors"
                            >Cancelar</button
                        >
                    {/if}
                </div>
                {#if formError}
                    <p class="text-[11px] text-red-500 mb-2">{formError}</p>
                {/if}
                <div class="flex gap-2 items-start">
                    <input
                        type="text"
                        bind:value={formNombre}
                        placeholder="Nombre…"
                        maxlength="24"
                        onkeydown={handleKeyEtiqueta}
                        class="flex-1 h-8 px-2 text-[13px] border border-slate-200 rounded-lg
							focus:outline-none focus:ring-1 focus:ring-navy/30 focus:border-navy/40 transition-colors"
                    />
                    <button
                        onclick={guardarEtiqueta}
                        disabled={formSaving || !formNombre.trim()}
                        class="h-8 px-3 rounded-lg bg-navy text-white text-[12px] font-medium
							hover:opacity-90 active:opacity-80 disabled:opacity-40 transition-opacity whitespace-nowrap"
                        >{editingEtiqueta ? "Guardar" : "Crear"}</button
                    >
                </div>
                <!-- Paleta de colores -->
                <div class="flex gap-1.5 mt-2 flex-wrap">
                    {#each ETIQUETA_COLORS as c}
                        <button
                            onclick={() => (formColor = c.hex)}
                            title={c.name}
                            class="w-5 h-5 rounded-full border-2 transition-all flex-shrink-0
								{formColor === c.hex
                                ? 'border-slate-600 scale-125'
                                : 'border-transparent hover:scale-110'}"
                            style="background:{c.hex};"
                        ></button>
                    {/each}
                </div>
                <!-- Vista previa -->
                {#if formNombre.trim()}
                    <div class="mt-2 flex items-center gap-2">
                        <span
                            class="text-[10px] text-slate-400 uppercase tracking-wider"
                            >Vista previa:</span
                        >
                        <span
                            style={etiquetaBadgeStyle(formColor)}
                            class="inline-block text-[11px] font-semibold px-1.5 py-0.5 rounded border font-mono leading-none"
                            >{formNombre}</span
                        >
                    </div>
                {/if}
            </div>
        </div>
    </div>
{/if}

<!-- ── Modal de importación XLSX ────────────────────────────────── -->
{#if xlsxModalOpen}
    <div
        class="fixed inset-0 bg-black/40 z-50 flex items-end sm:items-center justify-center p-4"
        onclick={cerrarModalXlsx}
        onkeydown={(e) => e.key === "Escape" && cerrarModalXlsx()}
        role="dialog"
        aria-modal="true"
        tabindex="-1"
    >
        <div
            class="bg-white rounded-xl shadow-2xl w-full max-w-2xl max-h-[85vh] flex flex-col overflow-hidden"
            onclick={(e) => e.stopPropagation()}
            onkeydown={(e) => e.stopPropagation()}
            role="document"
        >
            <!-- Header -->
            <div
                class="px-4 pt-4 pb-3 border-b border-slate-100 flex items-center justify-between flex-shrink-0"
            >
                <div>
                    <p
                        class="text-[11px] font-semibold uppercase tracking-wider text-slate-400"
                    >
                        XLSX
                    </p>
                    <h2
                        class="font-barlow-condensed text-[17px] font-bold text-navy"
                    >
                        Importar emparejamientos
                    </h2>
                </div>
                <button
                    onclick={cerrarModalXlsx}
                    aria-label="Cerrar"
                    class="w-8 h-8 flex items-center justify-center rounded-lg text-slate-400
						hover:bg-slate-100 hover:text-slate-600 transition-colors"
                >
                    <svg
                        class="w-4 h-4"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                    >
                        <line x1="18" y1="6" x2="6" y2="18" /><line
                            x1="6"
                            y1="6"
                            x2="18"
                            y2="18"
                        />
                    </svg>
                </button>
            </div>

            <!-- Contenido -->
            {#if xlsxResult !== null}
                <!-- Estado de éxito -->
                <div
                    class="flex-1 flex flex-col items-center justify-center gap-3 py-12 px-6"
                >
                    <div
                        class="w-12 h-12 rounded-full bg-emerald-50 flex items-center justify-center"
                    >
                        <svg
                            class="w-6 h-6 text-emerald-500"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2.5"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        >
                            <polyline points="20 6 9 17 4 12" />
                        </svg>
                    </div>
                    <p class="text-[15px] font-semibold text-slate-700">
                        Se importaron <span class="text-navy">{xlsxResult}</span
                        >
                        emparejamiento{xlsxResult !== 1 ? "s" : ""}
                    </p>
                    <button
                        onclick={cerrarModalXlsx}
                        class="mt-2 h-9 px-5 rounded-lg bg-navy text-white text-[13px] font-medium
							hover:opacity-90 active:opacity-80 transition-opacity">Cerrar</button
                    >
                </div>
            {:else if xlsxPreview}
                <!-- Barra de resumen -->
                <div
                    class="px-4 py-2.5 bg-slate-50 border-b border-slate-100 flex items-center gap-3 flex-shrink-0 flex-wrap"
                >
                    <span class="text-[12px] text-slate-600 font-medium">
                        {xlsxPreview.total_rows} fila{xlsxPreview.total_rows !==
                        1
                            ? "s"
                            : ""} en el archivo
                    </span>
                    <span class="text-slate-300">·</span>
                    <span class="text-[12px] font-semibold text-emerald-600"
                        >{xlsxPreview.valid_count} válidas</span
                    >
                    {#if xlsxPreview.error_count > 0}
                        <span class="text-slate-300">·</span>
                        <span class="text-[12px] font-semibold text-red-500"
                            >{xlsxPreview.error_count} con errores</span
                        >
                    {/if}
                </div>

                <!-- Modo de importación -->
                <div class="px-4 py-3 border-b border-slate-100 flex-shrink-0">
                    <p
                        class="text-[11px] font-semibold uppercase tracking-wider text-slate-400 mb-2"
                    >
                        Modo de importación
                    </p>
                    <div class="flex gap-3">
                        <label class="flex items-center gap-2 cursor-pointer">
                            <input
                                type="radio"
                                bind:group={xlsxMode}
                                value="agregar"
                                class="accent-navy"
                            />
                            <span class="text-[13px] text-slate-700 font-medium"
                                >Agregar</span
                            >
                            <span class="text-[11px] text-slate-400"
                                >(conserva existentes, omite duplicados)</span
                            >
                        </label>
                        <label class="flex items-center gap-2 cursor-pointer">
                            <input
                                type="radio"
                                bind:group={xlsxMode}
                                value="reemplazar"
                                class="accent-red-500"
                            />
                            <span class="text-[13px] text-slate-700 font-medium"
                                >Reemplazar todo</span
                            >
                        </label>
                    </div>
                    {#if xlsxMode === "reemplazar"}
                        <div
                            class="mt-2 flex items-start gap-1.5 px-2.5 py-2 rounded-lg bg-red-50 border border-red-200"
                        >
                            <svg
                                class="w-3.5 h-3.5 text-red-500 flex-shrink-0 mt-0.5"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2.5"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <path
                                    d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"
                                />
                                <line x1="12" y1="9" x2="12" y2="13" /><line
                                    x1="12"
                                    y1="17"
                                    x2="12.01"
                                    y2="17"
                                />
                            </svg>
                            <p class="text-[11px] text-red-600">
                                Esto eliminará todos los emparejamientos
                                existentes y sus etiquetas antes de importar.
                            </p>
                        </div>
                    {/if}
                </div>

                {#if xlsxError}
                    <div
                        class="px-4 py-2 bg-red-50 border-b border-red-200 flex-shrink-0"
                    >
                        <p class="text-[12px] text-red-600">{xlsxError}</p>
                    </div>
                {/if}

                <!-- Tabla de previsualización -->
                <div class="overflow-y-auto flex-1">
                    <table class="w-full border-collapse text-[12px]">
                        <thead class="sticky top-0 bg-white z-10">
                            <tr class="border-b border-slate-200">
                                <th
                                    class="text-left px-3 py-2 text-[10px] font-semibold uppercase tracking-wider text-slate-400 w-8"
                                    >#</th
                                >
                                <th
                                    class="text-left px-3 py-2 text-[10px] font-semibold uppercase tracking-wider text-slate-400"
                                    >numart_origen</th
                                >
                                <th
                                    class="text-left px-3 py-2 text-[10px] font-semibold uppercase tracking-wider text-slate-400"
                                    >unidad_fraccion</th
                                >
                                <th
                                    class="text-left px-3 py-2 text-[10px] font-semibold uppercase tracking-wider text-slate-400"
                                    >numart_destino</th
                                >
                                <th
                                    class="text-left px-3 py-2 text-[10px] font-semibold uppercase tracking-wider text-slate-400"
                                    >Errores</th
                                >
                            </tr>
                        </thead>
                        <tbody>
                            {#each xlsxPreview.rows as row (row.row_index)}
                                {@const hasError = row.errors.length > 0}
                                <tr
                                    class="border-t border-slate-100 {hasError
                                        ? 'bg-red-50'
                                        : 'bg-emerald-50/40'}"
                                >
                                    <td
                                        class="px-3 py-2 text-slate-400 font-mono"
                                        >{row.row_index}</td
                                    >
                                    <td
                                        class="px-3 py-2 font-mono text-[11px] {hasError &&
                                        !row.numart_origen
                                            ? 'text-red-400 italic'
                                            : 'text-slate-700'}"
                                        >{row.numart_origen || "—"}</td
                                    >
                                    <td
                                        class="px-3 py-2 font-mono text-[11px] {hasError &&
                                        !row.unidad_fraccion
                                            ? 'text-red-400 italic'
                                            : 'text-slate-700'}"
                                        >{row.unidad_fraccion || "—"}</td
                                    >
                                    <td
                                        class="px-3 py-2 font-mono text-[11px] {hasError &&
                                        !row.numart_destino
                                            ? 'text-red-400 italic'
                                            : 'text-slate-700'}"
                                        >{row.numart_destino || "—"}</td
                                    >
                                    <td class="px-3 py-2">
                                        {#if hasError}
                                            <span
                                                class="text-[11px] text-red-600 font-medium"
                                                >{row.errors.join(", ")}</span
                                            >
                                        {:else}
                                            <span
                                                class="text-[11px] text-emerald-600"
                                                >✓</span
                                            >
                                        {/if}
                                    </td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                </div>

                <!-- Footer -->
                <div
                    class="px-4 py-3 border-t border-slate-100 flex items-center justify-between gap-3 flex-shrink-0"
                >
                    <p class="text-[12px] text-slate-500">
                        Se importarán <span class="font-semibold text-slate-700"
                            >{xlsxPreview.valid_count}</span
                        >
                        fila{xlsxPreview.valid_count !== 1 ? "s" : ""} válidas
                    </p>
                    <div class="flex gap-2">
                        <button
                            onclick={cerrarModalXlsx}
                            class="h-8 px-3 rounded-lg border border-slate-200 text-slate-600 text-[12px] font-medium
								hover:bg-slate-50 active:bg-slate-100 transition-colors">Cancelar</button
                        >
                        <button
                            onclick={confirmarImport}
                            disabled={xlsxPreview.valid_count === 0 ||
                                xlsxImporting}
                            class="h-8 px-4 rounded-lg text-[12px] font-medium transition-colors
								{xlsxMode === 'reemplazar'
                                ? 'bg-red-500 hover:bg-red-600 active:bg-red-700 text-white disabled:opacity-40'
                                : 'bg-navy hover:opacity-90 active:opacity-80 text-white disabled:opacity-40'}"
                        >
                            {xlsxImporting
                                ? "Importando…"
                                : `Importar ${xlsxPreview.valid_count} →`}
                        </button>
                    </div>
                </div>
            {:else}
                <!-- Estado de error sin preview -->
                <div
                    class="flex-1 flex flex-col items-center justify-center gap-3 py-12 px-6"
                >
                    {#if xlsxError}
                        <div
                            class="flex items-start gap-2 px-3 py-2.5 rounded-lg bg-red-50 border border-red-200 max-w-sm"
                        >
                            <p class="text-[13px] text-red-600">{xlsxError}</p>
                        </div>
                    {/if}
                    <button
                        onclick={cerrarModalXlsx}
                        class="h-9 px-5 rounded-lg border border-slate-200 text-slate-600 text-[13px] font-medium
							hover:bg-slate-50 transition-colors">Cerrar</button
                    >
                </div>
            {/if}
        </div>
    </div>
{/if}

<!-- ── Modal de nuevo emparejamiento ───────────────────────────── -->
{#if paso > 0}
    <div
        class="fixed inset-0 bg-black/40 z-50 flex items-end sm:items-center justify-center p-4"
        onclick={cerrarModal}
        onkeydown={(e) => e.key === "Escape" && cerrarModal()}
        role="dialog"
        aria-modal="true"
        tabindex="-1"
    >
        <div
            class="bg-white rounded-xl shadow-2xl w-full max-w-md max-h-[80vh] flex flex-col overflow-hidden"
            onclick={(e) => e.stopPropagation()}
            onkeydown={(e) => e.stopPropagation()}
            role="document"
        >
            <!-- Header del modal -->
            <div class="px-4 pt-4 pb-3 border-b border-slate-100">
                <div class="flex items-center justify-between mb-2">
                    <div class="flex items-center gap-2">
                        <!-- Indicador de pasos -->
                        {#each [1, 2, 3] as s}
                            <div class="flex items-center gap-1">
                                <div
                                    class="w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-bold
									{paso >= s ? 'bg-navy text-white' : 'bg-slate-100 text-slate-400'}"
                                >
                                    {s}
                                </div>
                                {#if s < 3}
                                    <div
                                        class="w-4 h-px {paso > s
                                            ? 'bg-navy'
                                            : 'bg-slate-200'}"
                                    ></div>
                                {/if}
                            </div>
                        {/each}
                    </div>
                    <button
                        onclick={cerrarModal}
                        aria-label="Cerrar"
                        class="w-8 h-8 flex items-center justify-center rounded-lg text-slate-400
							hover:bg-slate-100 hover:text-slate-600 transition-colors"
                    >
                        <svg
                            class="w-4 h-4"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                        >
                            <line x1="18" y1="6" x2="6" y2="18" /><line
                                x1="6"
                                y1="6"
                                x2="18"
                                y2="18"
                            />
                        </svg>
                    </button>
                </div>

                {#if paso === 1}
                    <p
                        class="text-[11px] font-semibold uppercase tracking-wider text-slate-400"
                    >
                        Paso 1 de 3
                    </p>
                    <p
                        class="font-barlow-condensed text-[17px] font-bold text-navy"
                    >
                        Buscar artículo origen
                    </p>
                    <div class="relative mt-3">
                        <svg
                            class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400 pointer-events-none"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.75"
                            stroke-linecap="round"
                        >
                            <circle cx="11" cy="11" r="8" /><line
                                x1="21"
                                y1="21"
                                x2="16.65"
                                y2="16.65"
                            />
                        </svg>
                        <input
                            type="text"
                            placeholder="Descripción o clave del artículo con fracciones…"
                            value={queryOrigen}
                            oninput={onQueryOrigenInput}
                            use:focusOnMount
                            class="w-full h-9 pl-9 pr-3 rounded-lg text-[13px] border border-slate-200 uppercase
								focus:outline-none focus:ring-2 focus:ring-navy/20 focus:border-navy/40 transition-colors"
                        />
                    </div>
                {:else if paso === 2}
                    <p
                        class="text-[11px] font-semibold uppercase tracking-wider text-slate-400"
                    >
                        Paso 2 de 3
                    </p>
                    <p
                        class="font-barlow-condensed text-[17px] font-bold text-navy"
                    >
                        Elegir fracción
                    </p>
                    {#if artOrigenActivo}
                        <p class="text-[12px] text-slate-500 mt-0.5 truncate">
                            {artOrigenActivo.desc}
                        </p>
                    {/if}
                {:else if paso === 3}
                    <p
                        class="text-[11px] font-semibold uppercase tracking-wider text-slate-400"
                    >
                        Paso 3 de 3
                    </p>
                    <p
                        class="font-barlow-condensed text-[17px] font-bold text-navy"
                    >
                        Buscar artículo destino
                        {#if fraccionActiva}<span
                                class="text-slate-400 font-normal text-[14px]"
                                >· fracc. {fraccionActiva.unidad}</span
                            >{/if}
                    </p>
                    <div class="relative mt-3">
                        <svg
                            class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400 pointer-events-none"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.75"
                            stroke-linecap="round"
                        >
                            <circle cx="11" cy="11" r="8" /><line
                                x1="21"
                                y1="21"
                                x2="16.65"
                                y2="16.65"
                            />
                        </svg>
                        <input
                            type="text"
                            placeholder="Descripción o clave del artículo a comparar…"
                            value={queryDestino}
                            oninput={onQueryDestinoInput}
                            use:focusOnMount
                            class="w-full h-9 pl-9 pr-3 rounded-lg text-[13px] border border-slate-200 uppercase
								focus:outline-none focus:ring-2 focus:ring-navy/20 focus:border-navy/40 transition-colors"
                        />
                    </div>
                {/if}
            </div>

            <!-- Cuerpo del modal -->
            <div class="overflow-y-auto flex-1">
                {#if paso === 1}
                    {#if !queryOrigen.trim()}
                        <p class="text-center text-[13px] text-slate-400 py-8">
                            Escribe para buscar
                        </p>
                    {:else if resultadosOrigen.length === 0}
                        <p class="text-center text-[13px] text-slate-400 py-8">
                            Sin resultados para "{queryOrigen}"
                        </p>
                    {:else}
                        {#each resultadosOrigen as art (art.numart)}
                            <button
                                onclick={() => seleccionarOrigen(art)}
                                class="w-full text-left px-4 py-3 border-b border-slate-50 hover:bg-sky-50 transition-colors"
                            >
                                <p
                                    class="text-[13px] font-semibold text-navy leading-snug"
                                >
                                    {art.desc}
                                </p>
                                <div class="flex items-center gap-2 mt-0.5">
                                    <span
                                        class="font-mono text-[11px] text-slate-400"
                                        >{art.numart}</span
                                    >
                                    <span class="text-slate-300">·</span>
                                    <span class="text-[11px] text-slate-500"
                                        >{art.fracciones.length} fracc.</span
                                    >
                                </div>
                            </button>
                        {/each}
                    {/if}
                {:else if paso === 2 && artOrigenActivo}
                    {#each artOrigenActivo.fracciones as frac (frac.unidad)}
                        {@const yaPareada = frac.pareado !== null}
                        <button
                            onclick={() =>
                                seleccionarFraccion(
                                    artOrigenActivo!.numart,
                                    frac.unidad,
                                )}
                            class="w-full text-left px-4 py-3 border-b border-slate-50 hover:bg-sky-50 transition-colors"
                        >
                            <div
                                class="flex items-center justify-between gap-2"
                            >
                                <span
                                    class="font-mono font-bold text-navy text-[14px]"
                                    >{frac.unidad}</span
                                >
                                {#if yaPareada}
                                    <span
                                        class="text-[10px] font-semibold text-amber/80 bg-amber/10 border border-amber/20 px-1.5 py-0.5 rounded"
                                    >
                                        Ya emparejada
                                    </span>
                                {/if}
                            </div>
                            {#if yaPareada && frac.pareado}
                                <p
                                    class="text-[11px] text-slate-400 mt-0.5 truncate"
                                >
                                    → {frac.pareado.desc}
                                </p>
                            {/if}
                        </button>
                    {/each}
                {:else if paso === 3}
                    {#if !queryDestino.trim()}
                        <p class="text-center text-[13px] text-slate-400 py-8">
                            Escribe para buscar
                        </p>
                    {:else if resultadosDestino.length === 0}
                        <p class="text-center text-[13px] text-slate-400 py-8">
                            Sin resultados para "{queryDestino}"
                        </p>
                    {:else}
                        {#each resultadosDestino as art (art.numart)}
                            <button
                                onclick={() => seleccionarDestino(art)}
                                class="w-full text-left px-4 py-3 border-b border-slate-50 hover:bg-sky-50 transition-colors"
                            >
                                <p
                                    class="text-[13px] font-semibold text-navy leading-snug"
                                >
                                    {art.desc}
                                </p>
                                <div class="flex items-center gap-2 mt-0.5">
                                    <span
                                        class="font-mono text-[11px] text-slate-400"
                                        >{art.numart}</span
                                    >
                                    {#if art.unidad}
                                        <span class="text-slate-300">·</span>
                                        <span class="text-[11px] text-slate-500"
                                            >{art.unidad}</span
                                        >
                                    {/if}
                                </div>
                            </button>
                        {/each}
                    {/if}
                {/if}
            </div>

            <!-- Pie: botón volver -->
            {#if paso > 1}
                <div class="px-4 py-3 border-t border-slate-100">
                    <button
                        onclick={() => {
                            paso = (paso - 1) as 1 | 2;
                        }}
                        class="text-[12px] font-medium text-slate-500 hover:text-slate-700 flex items-center gap-1 transition-colors"
                    >
                        <svg
                            class="w-3.5 h-3.5"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        >
                            <polyline points="15 18 9 12 15 6" />
                        </svg>
                        Volver
                    </button>
                </div>
            {/if}
        </div>
    </div>
{/if}

<!-- ── Modal de nuevo seguimiento (2 pasos) ────────────────────── -->
{#if pasoSeg > 0}
    <div
        class="fixed inset-0 bg-black/40 z-50 flex items-end sm:items-center justify-center p-4"
        onclick={cerrarModalSeg}
        onkeydown={(e) => e.key === "Escape" && cerrarModalSeg()}
        role="dialog"
        aria-modal="true"
        tabindex="-1"
    >
        <div
            class="bg-white rounded-xl shadow-2xl w-full max-w-md max-h-[80vh] flex flex-col overflow-hidden"
            onclick={(e) => e.stopPropagation()}
            onkeydown={(e) => e.stopPropagation()}
            role="document"
        >
            <div class="px-4 pt-4 pb-3 border-b border-slate-100">
                <div class="flex items-center justify-between mb-2">
                    <div class="flex items-center gap-2">
                        {#each [1, 2] as s}
                            <div class="flex items-center gap-1">
                                <div
                                    class="w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-bold
                                    {pasoSeg >= s
                                        ? 'bg-navy text-white'
                                        : 'bg-slate-100 text-slate-400'}"
                                >
                                    {s}
                                </div>
                                {#if s < 2}
                                    <div
                                        class="w-4 h-px {pasoSeg > s
                                            ? 'bg-navy'
                                            : 'bg-slate-200'}"
                                    ></div>
                                {/if}
                            </div>
                        {/each}
                    </div>
                    <button
                        onclick={cerrarModalSeg}
                        aria-label="Cerrar"
                        class="w-8 h-8 flex items-center justify-center rounded-lg text-slate-400
                                hover:bg-slate-100 hover:text-slate-600 transition-colors"
                    >
                        <svg
                            class="w-4 h-4"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                        >
                            <line x1="18" y1="6" x2="6" y2="18" /><line
                                x1="6"
                                y1="6"
                                x2="18"
                                y2="18"
                            />
                        </svg>
                    </button>
                </div>
                {#if pasoSeg === 1}
                    <p
                        class="text-[11px] font-semibold uppercase tracking-wider text-slate-400"
                    >
                        Paso 1 de 2
                    </p>
                    <p
                        class="font-barlow-condensed text-[17px] font-bold text-navy"
                    >
                        Buscar artículo
                    </p>
                    <div class="relative mt-3">
                        <svg
                            class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400 pointer-events-none"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.75"
                            stroke-linecap="round"
                        >
                            <circle cx="11" cy="11" r="8" /><line
                                x1="21"
                                y1="21"
                                x2="16.65"
                                y2="16.65"
                            />
                        </svg>
                        <input
                            type="text"
                            placeholder="Descripción o clave del artículo con fracciones…"
                            value={queryOrigenSeg}
                            oninput={onQueryOrigenSegInput}
                            use:focusOnMount
                            class="w-full h-9 pl-9 pr-3 rounded-lg text-[13px] border border-slate-200 uppercase
                                    focus:outline-none focus:ring-2 focus:ring-navy/20 focus:border-navy/40 transition-colors"
                        />
                    </div>
                {:else if pasoSeg === 2}
                    <p
                        class="text-[11px] font-semibold uppercase tracking-wider text-slate-400"
                    >
                        Paso 2 de 2
                    </p>
                    <p
                        class="font-barlow-condensed text-[17px] font-bold text-navy"
                    >
                        Elegir fracción
                    </p>
                    {#if artOrigenActivoSeg}
                        <p class="text-[12px] text-slate-500 mt-0.5 truncate">
                            {artOrigenActivoSeg.desc}
                        </p>
                    {/if}
                {/if}
            </div>
            <div class="overflow-y-auto flex-1">
                {#if pasoSeg === 1}
                    {#if !queryOrigenSeg.trim()}
                        <p class="text-center text-[13px] text-slate-400 py-8">
                            Escribe para buscar
                        </p>
                    {:else if resultadosOrigenSeg.length === 0}
                        <p class="text-center text-[13px] text-slate-400 py-8">
                            Sin resultados para "{queryOrigenSeg}"
                        </p>
                    {:else}
                        {#each resultadosOrigenSeg as art (art.numart)}
                            <button
                                onclick={() => seleccionarOrigenSeg(art)}
                                class="w-full text-left px-4 py-3 border-b border-slate-50 hover:bg-sky-50 transition-colors"
                            >
                                <p
                                    class="text-[13px] font-semibold text-navy leading-snug"
                                >
                                    {art.desc}
                                </p>
                                <div class="flex items-center gap-2 mt-0.5">
                                    <span
                                        class="font-mono text-[11px] text-slate-400"
                                        >{art.numart}</span
                                    >
                                    <span class="text-slate-300">·</span>
                                    <span class="text-[11px] text-slate-500"
                                        >{art.fracciones.length} fracc.</span
                                    >
                                </div>
                            </button>
                        {/each}
                    {/if}
                {:else if pasoSeg === 2 && artOrigenActivoSeg}
                    {#each artOrigenActivoSeg.fracciones as frac (frac.unidad)}
                        {@const yaEnSeg = seguimientosSet.has(
                            `${artOrigenActivoSeg.numart}|${frac.unidad}`,
                        )}
                        <button
                            onclick={() =>
                                seleccionarFraccionSeg(
                                    artOrigenActivoSeg!.numart,
                                    frac.unidad,
                                )}
                            disabled={yaEnSeg}
                            class="w-full text-left px-4 py-3 border-b border-slate-50 transition-colors
                                    {yaEnSeg
                                ? 'opacity-50 cursor-not-allowed'
                                : 'hover:bg-sky-50'}"
                        >
                            <div
                                class="flex items-center justify-between gap-2"
                            >
                                <span
                                    class="font-mono font-bold text-navy text-[14px]"
                                    >{frac.unidad}</span
                                >
                                {#if yaEnSeg}
                                    <span
                                        class="text-[10px] font-semibold text-slate-500 bg-slate-100 border border-slate-200 px-1.5 py-0.5 rounded"
                                    >
                                        Ya en seguimiento
                                    </span>
                                {/if}
                            </div>
                            <p class="text-[11px] text-slate-400 mt-0.5">
                                Factor {frac.equiv2}/{frac.equiv1} = {(frac.equiv1 >
                                0
                                    ? frac.equiv2 / frac.equiv1
                                    : 1
                                ).toFixed(3)}×
                            </p>
                        </button>
                    {/each}
                {/if}
            </div>
            {#if pasoSeg > 1}
                <div class="px-4 py-3 border-t border-slate-100">
                    <button
                        onclick={() => {
                            pasoSeg = 1;
                        }}
                        class="text-[12px] font-medium text-slate-500 hover:text-slate-700 flex items-center gap-1 transition-colors"
                    >
                        <svg
                            class="w-3.5 h-3.5"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        >
                            <polyline points="15 18 9 12 15 6" />
                        </svg>
                        Volver
                    </button>
                </div>
            {/if}
        </div>
    </div>
{/if}

<!-- ── Modal XLSX seguimientos ──────────────────────────────────── -->
{#if xlsxSegModalOpen}
    <div
        class="fixed inset-0 bg-black/40 z-50 flex items-end sm:items-center justify-center p-4"
        onclick={cerrarModalXlsxSeg}
        onkeydown={(e) => e.key === "Escape" && cerrarModalXlsxSeg()}
        role="dialog"
        aria-modal="true"
        tabindex="-1"
    >
        <div
            class="bg-white rounded-xl shadow-2xl w-full max-w-xl max-h-[85vh] flex flex-col overflow-hidden"
            onclick={(e) => e.stopPropagation()}
            onkeydown={(e) => e.stopPropagation()}
            role="document"
        >
            <div
                class="px-4 pt-4 pb-3 border-b border-slate-100 flex items-center justify-between flex-shrink-0"
            >
                <div>
                    <p
                        class="text-[11px] font-semibold uppercase tracking-wider text-slate-400"
                    >
                        XLSX
                    </p>
                    <h2
                        class="font-barlow-condensed text-[17px] font-bold text-navy"
                    >
                        Importar seguimientos
                    </h2>
                </div>
                <button
                    onclick={cerrarModalXlsxSeg}
                    aria-label="Cerrar"
                    class="w-8 h-8 flex items-center justify-center rounded-lg text-slate-400
                            hover:bg-slate-100 hover:text-slate-600 transition-colors"
                >
                    <svg
                        class="w-4 h-4"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                    >
                        <line x1="18" y1="6" x2="6" y2="18" /><line
                            x1="6"
                            y1="6"
                            x2="18"
                            y2="18"
                        />
                    </svg>
                </button>
            </div>
            {#if xlsxSegResult !== null}
                <div
                    class="flex-1 flex flex-col items-center justify-center gap-3 py-12 px-6"
                >
                    <div
                        class="w-12 h-12 rounded-full bg-emerald-50 flex items-center justify-center"
                    >
                        <svg
                            class="w-6 h-6 text-emerald-500"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2.5"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        >
                            <polyline points="20 6 9 17 4 12" />
                        </svg>
                    </div>
                    <p class="text-[15px] font-semibold text-slate-700">
                        Se importaron <span class="text-navy"
                            >{xlsxSegResult}</span
                        >
                        seguimiento{xlsxSegResult !== 1 ? "s" : ""}
                    </p>
                    <button
                        onclick={cerrarModalXlsxSeg}
                        class="mt-2 h-9 px-5 rounded-lg bg-navy text-white text-[13px] font-medium
                                hover:opacity-90 active:opacity-80 transition-opacity"
                        >Cerrar</button
                    >
                </div>
            {:else if xlsxSegPreview}
                <div
                    class="px-4 py-2.5 bg-slate-50 border-b border-slate-100 flex items-center gap-3 flex-shrink-0 flex-wrap"
                >
                    <span class="text-[12px] text-slate-600 font-medium">
                        {xlsxSegPreview.total_rows} fila{xlsxSegPreview.total_rows !==
                        1
                            ? "s"
                            : ""} en el archivo
                    </span>
                    <span class="text-slate-300">·</span>
                    <span class="text-[12px] font-semibold text-emerald-600"
                        >{xlsxSegPreview.valid_count} válidas</span
                    >
                    {#if xlsxSegPreview.error_count > 0}
                        <span class="text-slate-300">·</span>
                        <span class="text-[12px] font-semibold text-red-500"
                            >{xlsxSegPreview.error_count} con errores</span
                        >
                    {/if}
                </div>
                <div class="px-4 py-3 border-b border-slate-100 flex-shrink-0">
                    <p
                        class="text-[11px] font-semibold uppercase tracking-wider text-slate-400 mb-2"
                    >
                        Modo de importación
                    </p>
                    <div class="flex gap-3">
                        <label class="flex items-center gap-2 cursor-pointer">
                            <input
                                type="radio"
                                bind:group={xlsxSegMode}
                                value="agregar"
                                class="accent-navy"
                            />
                            <span class="text-[13px] text-slate-700 font-medium"
                                >Agregar</span
                            >
                            <span class="text-[11px] text-slate-400"
                                >(conserva existentes, omite duplicados)</span
                            >
                        </label>
                        <label class="flex items-center gap-2 cursor-pointer">
                            <input
                                type="radio"
                                bind:group={xlsxSegMode}
                                value="reemplazar"
                                class="accent-red-500"
                            />
                            <span class="text-[13px] text-slate-700 font-medium"
                                >Reemplazar todo</span
                            >
                        </label>
                    </div>
                    {#if xlsxSegMode === "reemplazar"}
                        <div
                            class="mt-2 flex items-start gap-1.5 px-2.5 py-2 rounded-lg bg-red-50 border border-red-200"
                        >
                            <svg
                                class="w-3.5 h-3.5 text-red-500 flex-shrink-0 mt-0.5"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2.5"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <path
                                    d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"
                                />
                                <line x1="12" y1="9" x2="12" y2="13" /><line
                                    x1="12"
                                    y1="17"
                                    x2="12.01"
                                    y2="17"
                                />
                            </svg>
                            <p class="text-[11px] text-red-600">
                                Esto eliminará todos los seguimientos existentes
                                antes de importar.
                            </p>
                        </div>
                    {/if}
                </div>
                {#if xlsxSegError}
                    <div
                        class="px-4 py-2 bg-red-50 border-b border-red-200 flex-shrink-0"
                    >
                        <p class="text-[12px] text-red-600">{xlsxSegError}</p>
                    </div>
                {/if}
                <div class="overflow-y-auto flex-1">
                    <table class="w-full border-collapse text-[12px]">
                        <thead class="sticky top-0 bg-white z-10">
                            <tr class="border-b border-slate-200">
                                <th
                                    class="text-left px-3 py-2 text-[10px] font-semibold uppercase tracking-wider text-slate-400 w-8"
                                    >#</th
                                >
                                <th
                                    class="text-left px-3 py-2 text-[10px] font-semibold uppercase tracking-wider text-slate-400"
                                    >numart_origen</th
                                >
                                <th
                                    class="text-left px-3 py-2 text-[10px] font-semibold uppercase tracking-wider text-slate-400"
                                    >unidad_fraccion</th
                                >
                                <th
                                    class="text-left px-3 py-2 text-[10px] font-semibold uppercase tracking-wider text-slate-400"
                                    >Errores</th
                                >
                            </tr>
                        </thead>
                        <tbody>
                            {#each xlsxSegPreview.rows as row (row.row_index)}
                                {@const hasError = row.errors.length > 0}
                                <tr
                                    class="border-t border-slate-100 {hasError
                                        ? 'bg-red-50'
                                        : 'bg-emerald-50/40'}"
                                >
                                    <td
                                        class="px-3 py-2 text-slate-400 font-mono"
                                        >{row.row_index}</td
                                    >
                                    <td
                                        class="px-3 py-2 font-mono text-[11px] {hasError &&
                                        !row.numart_origen
                                            ? 'text-red-400 italic'
                                            : 'text-slate-700'}"
                                        >{row.numart_origen || "—"}</td
                                    >
                                    <td
                                        class="px-3 py-2 font-mono text-[11px] {hasError &&
                                        !row.unidad_fraccion
                                            ? 'text-red-400 italic'
                                            : 'text-slate-700'}"
                                        >{row.unidad_fraccion || "—"}</td
                                    >
                                    <td class="px-3 py-2">
                                        {#if hasError}
                                            <span
                                                class="text-[11px] text-red-600 font-medium"
                                                >{row.errors.join(", ")}</span
                                            >
                                        {:else}
                                            <span
                                                class="text-[11px] text-emerald-600"
                                                >✓</span
                                            >
                                        {/if}
                                    </td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                </div>
                <div
                    class="px-4 py-3 border-t border-slate-100 flex items-center justify-between gap-3 flex-shrink-0"
                >
                    <p class="text-[12px] text-slate-500">
                        Se importarán <span class="font-semibold text-slate-700"
                            >{xlsxSegPreview.valid_count}</span
                        >
                        fila{xlsxSegPreview.valid_count !== 1 ? "s" : ""} válidas
                    </p>
                    <div class="flex gap-2">
                        <button
                            onclick={cerrarModalXlsxSeg}
                            class="h-8 px-3 rounded-lg border border-slate-200 text-slate-600 text-[12px] font-medium
                                    hover:bg-slate-50 active:bg-slate-100 transition-colors"
                            >Cancelar</button
                        >
                        <button
                            onclick={confirmarImportSeg}
                            disabled={xlsxSegPreview.valid_count === 0 ||
                                xlsxSegImporting}
                            class="h-8 px-4 rounded-lg text-[12px] font-medium transition-colors
                                    {xlsxSegMode === 'reemplazar'
                                ? 'bg-red-500 hover:bg-red-600 active:bg-red-700 text-white disabled:opacity-40'
                                : 'bg-navy hover:opacity-90 active:opacity-80 text-white disabled:opacity-40'}"
                        >
                            {xlsxSegImporting
                                ? "Importando…"
                                : `Importar ${xlsxSegPreview.valid_count} →`}
                        </button>
                    </div>
                </div>
            {:else}
                <div
                    class="flex-1 flex flex-col items-center justify-center gap-3 py-12 px-6"
                >
                    {#if xlsxSegError}
                        <div
                            class="flex items-start gap-2 px-3 py-2.5 rounded-lg bg-red-50 border border-red-200 max-w-sm"
                        >
                            <p class="text-[13px] text-red-600">
                                {xlsxSegError}
                            </p>
                        </div>
                    {/if}
                    <button
                        onclick={cerrarModalXlsxSeg}
                        class="h-9 px-5 rounded-lg border border-slate-200 text-slate-600 text-[13px] font-medium
                                hover:bg-slate-50 transition-colors"
                        >Cerrar</button
                    >
                </div>
            {/if}
        </div>
    </div>
{/if}
