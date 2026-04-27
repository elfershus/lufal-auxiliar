use anyhow::{Context, Result};
use tonic::metadata::MetadataValue;
use tonic::service::interceptor::InterceptedService;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint};
use tonic::{Request, Status};

use crate::types::*;

// Incluye el código generado por tonic-build desde vfpsync.proto
pub mod proto {
    tonic::include_proto!("vfpsync.v1");
}

use proto::vfp_sync_service_client::VfpSyncServiceClient;
use proto::{
    BuscarComprasRequest, BuscarRemisionesRequest, GetArticuloRequest,
    GetDocumentoRequest, GetProveedorRequest, ListAlmacenesRequest, ListDocumentosRequest,
};

// ── Interceptor de API key ─────────────────────────────────────

#[derive(Clone)]
pub struct ApiKeyInterceptor(pub String);

impl tonic::service::Interceptor for ApiKeyInterceptor {
    fn call(&mut self, mut req: Request<()>) -> std::result::Result<Request<()>, Status> {
        if let Ok(val) = self.0.parse::<MetadataValue<tonic::metadata::Ascii>>() {
            req.metadata_mut().insert("x-api-key", val);
        }
        Ok(req)
    }
}

type AuthClient = VfpSyncServiceClient<InterceptedService<Channel, ApiKeyInterceptor>>;

// ── Cliente gRPC ───────────────────────────────────────────────

pub struct GrpcClient {
    client: AuthClient,
}

impl GrpcClient {
    /// Crea el cliente con conexión diferida (connect_lazy).
    /// La conexión real ocurre en el primer RPC, dentro del runtime de Tauri.
    pub fn new(endpoint: &str, api_key: &str) -> Result<Self> {
        let tls = ClientTlsConfig::new().with_native_roots();
        let channel = Endpoint::from_shared(endpoint.to_string())
            .context("Endpoint inválido")?
            .tls_config(tls)
            .context("Error configurando TLS")?
            .connect_timeout(std::time::Duration::from_secs(8))
            .connect_lazy();

        let interceptor = ApiKeyInterceptor(api_key.to_string());
        let client = VfpSyncServiceClient::with_interceptor(channel, interceptor);
        Ok(GrpcClient { client })
    }

    // ── Almacenes ──────────────────────────────────────────────

    pub async fn list_almacenes(&mut self) -> Result<Vec<AlmacenRecord>> {
        let req = ListAlmacenesRequest {
            page_size: 200,
            page_token: String::new(),
            q: None,
        };
        let resp = self.client.list_almacenes(req).await?;
        Ok(resp
            .into_inner()
            .almacenes
            .into_iter()
            .map(|a| AlmacenRecord {
                numalm: a.numalm.trim().to_string(),
                nomalm: a.nomalm.trim().to_string(),
            })
            .collect())
    }

    // ── Documentos ─────────────────────────────────────────────

    pub async fn list_documentos(
        &mut self,
        params: ListDocumentosParams,
    ) -> Result<ListDocumentosResult> {
        let req = ListDocumentosRequest {
            tipodoc: params.tipodoc,
            numalm: params.numalm,
            fecha_from: params.fecha_from,
            fecha_to: params.fecha_to,
            fechacapt_from: params.fechacapt_from,
            fechacapt_to: params.fechacapt_to,
            numdoc: params.numdoc,
            status: params.status,
            page_size: params.page_size.unwrap_or(25),
            page_token: params.page_token.unwrap_or_default(),
            order_by: params.order_by,
            numcli: None,
            numprov: None,
            formapago: None,
        };
        let resp = self.client.list_documentos(req).await?;
        let inner = resp.into_inner();
        Ok(ListDocumentosResult {
            documentos: inner.documentos.into_iter().map(proto_doc_to_record).collect(),
            next_page_token: inner.next_page_token,
        })
    }

    // ── Detalle de documento ───────────────────────────────────

    pub async fn get_documento(
        &mut self,
        tipodoc: String,
        numdoc: String,
    ) -> Result<GetDocumentoResult> {
        let resp = self
            .client
            .get_documento(GetDocumentoRequest { tipodoc, numdoc })
            .await?;
        let inner = resp.into_inner();
        let documento = proto_doc_to_record(inner.documento.unwrap_or_default());
        let movimientos = inner.movimientos.into_iter().map(proto_mov_to_record).collect();
        Ok(GetDocumentoResult {
            documento,
            movimientos,
        })
    }

    // ── Nombre de proveedor ────────────────────────────────────

    pub async fn get_proveedor_nombre(&mut self, numprov: String) -> Result<String> {
        let resp = self
            .client
            .get_proveedor(GetProveedorRequest { numprov })
            .await?;
        Ok(resp
            .into_inner()
            .proveedor
            .map(|p| p.nomprov.trim().to_string())
            .unwrap_or_default())
    }

    // ── Artículos ──────────────────────────────────────────────

    pub async fn get_articulos(&mut self, numarts: Vec<String>) -> Result<Vec<ArticuloInfo>> {
        let mut set = tokio::task::JoinSet::new();
        for numart in numarts {
            let mut client = self.client.clone();
            set.spawn(async move {
                let resp = client
                    .get_articulo(GetArticuloRequest { numart: numart.clone() })
                    .await;
                resp.ok()
                    .and_then(|r| r.into_inner().articulo)
                    .map(|a| ArticuloInfo {
                        numart,
                        desc: a.desc.trim().to_string(),
                        unidad: a.unidad.trim().to_string(),
                    })
            });
        }
        let mut result = Vec::new();
        while let Some(res) = set.join_next().await {
            if let Ok(Some(info)) = res {
                result.push(info);
            }
        }
        Ok(result)
    }

    // ── Seguimiento ────────────────────────────────────────────

    pub async fn buscar_seguimiento(
        &mut self,
        numarts: Vec<String>,
        fecha_desde: String,
        numalm: String,
    ) -> Result<SeguimientoResult> {
        let mut client2 = self.client.clone();

        let (compras_resp, remisiones_resp) = tokio::join!(
            self.client.buscar_compras_por_articulos(BuscarComprasRequest {
                numarts: numarts.clone(),
                fecha_desde: fecha_desde.clone(),
                umbral_cobertura: 0.1,
            }),
            client2.buscar_remisiones_por_articulos(BuscarRemisionesRequest {
                numarts,
                fecha_desde,
                umbral_cobertura: 0.1,
                numalm,
            })
        );

        let compras = compras_resp?
            .into_inner()
            .items
            .into_iter()
            .map(|c| CompraMatchItem {
                tipodoc: c.tipodoc.trim().to_string(),
                numdoc: c.numdoc.trim().to_string(),
                fecha: c.fecha,
                numprov: c.numprov.trim().to_string(),
                importe: c.importe,
                status: c.status,
                arts_matched: c.arts_matched,
                total_arts: c.total_arts,
                coverage_pct: c.coverage_pct,
            })
            .collect();

        let remisiones = remisiones_resp?
            .into_inner()
            .items
            .into_iter()
            .map(|r| RemisionMatchItem {
                tipodoc: r.tipodoc.trim().to_string(),
                numdoc: r.numdoc.trim().to_string(),
                fecha: r.fecha,
                numcli: r.numcli.trim().to_string(),
                importe: r.importe,
                status: r.status,
                arts_matched: r.arts_matched,
                total_arts: r.total_arts,
                coverage_pct: r.coverage_pct,
            })
            .collect();

        Ok(SeguimientoResult { compras, remisiones })
    }
}

// ── Conversores proto → tipos locales ─────────────────────────

fn proto_doc_to_record(d: proto::DocumentoRecord) -> DocumentoRecord {
    DocumentoRecord {
        tipodoc: d.tipodoc.trim().to_string(),
        numdoc: d.numdoc.trim().to_string(),
        numalm: d.numalm.trim().to_string(),
        fecha: d.fecha,
        numprov: d.numprov.trim().to_string(),
        refer: d.refer.trim().to_string(),
        importe: d.importe,
        descuento: d.descuento,
        impuesto1: d.impuesto1,
        status: d.status,
        fechacapt: d.fechacapt,
        formapago: d.formapago.trim().to_string(),
        pjedesc: d.pjedesc,
        fechapago: d.fechapago,
        uuid: d.uuid.trim().to_string(),
        costo: d.costo,
    }
}

fn proto_mov_to_record(m: proto::MovimientoRecord) -> MovimientoRecord {
    MovimientoRecord {
        tipodoc: m.tipodoc.trim().to_string(),
        numdoc: m.numdoc.trim().to_string(),
        numpar: m.numpar.trim().to_string(),
        numart: m.numart.trim().to_string(),
        precio: m.precio,
        cant: m.cant,
        pend: m.pend,
        pjedesc: m.pjedesc,
        impuesto1: m.impuesto1,
        impuesto2: m.impuesto2,
        unidad: m.unidad.trim().to_string(),
        docant: m.docant.trim().to_string(),
    }
}
