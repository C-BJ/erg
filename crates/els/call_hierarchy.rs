use std::str::FromStr;

use erg_compiler::artifact::BuildRunnable;
use erg_compiler::erg_parser::parse::Parsable;

use erg_compiler::hir::Def;
use erg_compiler::varinfo::{AbsLocation, VarInfo};
use lsp_types::{
    CallHierarchyIncomingCall, CallHierarchyIncomingCallsParams, CallHierarchyItem,
    CallHierarchyOutgoingCall, CallHierarchyOutgoingCallsParams, CallHierarchyPrepareParams,
};

use crate::_log;
use crate::server::{ELSResult, RedirectableStdout, Server};
use crate::symbol::symbol_kind;
use crate::util::{abs_loc_to_lsp_loc, loc_to_pos, NormalizedUrl};

fn hierarchy_item(name: String, vi: &VarInfo) -> Option<CallHierarchyItem> {
    let loc = abs_loc_to_lsp_loc(&vi.def_loc)?;
    Some(CallHierarchyItem {
        name,
        kind: symbol_kind(vi),
        tags: None,
        detail: Some(vi.t.to_string()),
        uri: loc.uri,
        range: loc.range,
        selection_range: loc.range,
        data: Some(vi.def_loc.to_string().into()),
    })
}

impl<Checker: BuildRunnable, Parser: Parsable> Server<Checker, Parser> {
    pub(crate) fn handle_call_hierarchy_incoming(
        &mut self,
        params: CallHierarchyIncomingCallsParams,
    ) -> ELSResult<Option<Vec<CallHierarchyIncomingCall>>> {
        let mut res = vec![];
        _log!(self, "call hierarchy incoming calls requested: {params:?}");
        let Some(data) = params.item.data.as_ref().and_then(|d| d.as_str()) else {
            return Ok(None);
        };
        let Ok(loc) = AbsLocation::from_str(data) else {
            return Ok(None);
        };
        let Some(shared) = self.get_shared() else {
            return Ok(None);
        };
        if let Some(refs) = shared.index.get_refs(&loc) {
            for referrer_loc in refs.referrers.iter() {
                let Some(uri) = referrer_loc
                    .module
                    .as_ref()
                    .and_then(|path| NormalizedUrl::from_file_path(path).ok())
                else {
                    continue;
                };
                let Some(pos) = loc_to_pos(referrer_loc.loc) else {
                    continue;
                };
                if let Some(def) = self.get_min::<Def>(&uri, pos) {
                    if def.sig.is_subr() {
                        let Some(from) =
                            hierarchy_item(def.sig.inspect().to_string(), &def.sig.ident().vi)
                        else {
                            continue;
                        };
                        let call = CallHierarchyIncomingCall {
                            from,
                            from_ranges: vec![],
                        };
                        res.push(call);
                    }
                }
            }
        }
        Ok(Some(res))
    }

    pub(crate) fn handle_call_hierarchy_outgoing(
        &mut self,
        params: CallHierarchyOutgoingCallsParams,
    ) -> ELSResult<Option<Vec<CallHierarchyOutgoingCall>>> {
        _log!(self, "call hierarchy outgoing calls requested: {params:?}");
        Ok(None)
    }

    pub(crate) fn handle_call_hierarchy_prepare(
        &mut self,
        params: CallHierarchyPrepareParams,
    ) -> ELSResult<Option<Vec<CallHierarchyItem>>> {
        _log!(self, "call hierarchy prepare requested: {params:?}");
        let mut res = vec![];
        let uri = NormalizedUrl::new(params.text_document_position_params.text_document.uri);
        let pos = params.text_document_position_params.position;
        if let Some(token) = self.file_cache.get_symbol(&uri, pos) {
            if let Some(vi) = self.get_definition(&uri, &token)? {
                let Some(item) = hierarchy_item(token.content.to_string(), &vi) else {
                    return Ok(None);
                };
                res.push(item);
            }
        }
        Ok(Some(res))
    }
}
