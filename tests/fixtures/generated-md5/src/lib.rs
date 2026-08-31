include!(env!("CALCIT_BINDINGS_PATH"));

struct Service;

impl CalcitStdFfi for Service {
    fn calcit_std_hash_md5(&self, arg0: String) -> Result<String, String> {
        Ok(format!("{:x}", md5::compute(arg0)))
    }
}

static SERVICE: Service = Service;
export_calcit_std_ffi!(SERVICE);
