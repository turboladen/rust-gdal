use std::ptr::null;
use std::ffi::CString;
use libc::{c_void, c_int};
use vector::{Feature, Geometry, FieldValue};
use vector::defn::Defn;
use gdal_major_object::MajorObject;
use metadata::Metadata;
use gdal_sys::{ogr, ogr_enums};
use utils::{_last_null_pointer_err, _string};

use errors::*;

/// Layer in a vector dataset
///
/// ```
/// use std::path::Path;
/// use gdal::vector::Dataset;
///
/// let mut dataset = Dataset::open(Path::new("fixtures/roads.geojson")).unwrap();
/// let layer = dataset.layer(0).unwrap();
/// for feature in layer.features() {
///     // do something with each feature
/// }
/// ```
pub struct Layer {
    c_layer: *const c_void,
    defn: Defn,
}

impl MajorObject for Layer {
    unsafe fn gdal_object_ptr(&self) -> *const c_void {
        self.c_layer
    }
}

impl Metadata for Layer {}


impl Layer {
    pub unsafe fn _with_c_layer(c_layer: *const c_void) -> Layer {
        let c_defn = ogr::OGR_L_GetLayerDefn(c_layer);
        let defn = Defn::_with_c_defn(c_defn);
        return Layer{c_layer: c_layer, defn: defn};
    }

    /// Iterate over all features in this layer.
    pub fn features<'a>(&'a self) -> FeatureIterator<'a> {
        return FeatureIterator::_with_layer(&self);
    }

    pub fn set_spatial_filter(&self, geometry: &Geometry) {
        unsafe { ogr::OGR_L_SetSpatialFilter(self.c_layer, geometry.c_geometry()) };
    }

    pub fn clear_spatial_filter(&self) {
        unsafe { ogr::OGR_L_SetSpatialFilter(self.c_layer, null()) };
    }

    /// Get the name of this layer.
    pub fn name(&self) -> String {
        let rv = unsafe { ogr::OGR_L_GetName(self.c_layer) };
        return _string(rv);
    }

    pub fn defn(&self) -> &Defn {
        &self.defn
    }

    pub fn create_defn_fields(&self, fields_def: &[(&str, ogr_enums::OGRFieldType)]) -> Result<()> {
        for fd in fields_def {
            let fdefn = FieldDefn::new(fd.0, fd.1)?;
            fdefn.add_to_layer(self)?;
        }
        Ok(())
    }
    pub fn create_feature(&mut self, geometry: Geometry) -> Result<()> {
        let c_feature = unsafe { ogr::OGR_F_Create(self.defn.c_defn()) };
        let c_geometry = unsafe { geometry.into_c_geometry() };
        let rv = unsafe { ogr::OGR_F_SetGeometryDirectly(c_feature, c_geometry) };
        if rv != ogr_enums::OGRErr::OGRERR_NONE {
            return Err(ErrorKind::OgrError(rv, "OGR_F_SetGeometryDirectly").into());
        }
        let rv = unsafe { ogr::OGR_L_CreateFeature(self.c_layer, c_feature) };
        if rv != ogr_enums::OGRErr::OGRERR_NONE {
            return Err(ErrorKind::OgrError(rv, "OGR_L_CreateFeature").into());
        }
        Ok(())
    }

    pub fn create_feature_fields(&mut self, geometry: Geometry,
                                 field_names: &[&str], values: &[FieldValue]) -> Result<()> {
        let mut ft = Feature::new(&self.defn)?;
        ft.set_geometry(geometry)?;
        for (fd, ref val) in field_names.iter().zip(values.iter()) {
            ft.set_field(fd, val)?;
        }
        ft.create(self)?;
        Ok(())
    }
}

pub struct FeatureIterator<'a> {
    layer: &'a Layer,
}

impl<'a> Iterator for FeatureIterator<'a> {
    type Item = Feature<'a>;

    #[inline]
    fn next(&mut self) -> Option<Feature<'a>> {
        let c_feature = unsafe { ogr::OGR_L_GetNextFeature(self.layer.c_layer) };
        return match c_feature.is_null() {
            true  => None,
            false => Some(unsafe { Feature::_with_c_feature(self.layer.defn(), c_feature) }),
        };
    }
}

impl<'a> FeatureIterator<'a> {
    pub fn _with_layer(layer: &'a Layer) -> FeatureIterator<'a> {
        return FeatureIterator{layer: layer};
    }
}

pub struct FieldDefn {
    c_obj: *const c_void,
}

impl Drop for FieldDefn {
    fn drop(&mut self){
        unsafe { ogr::OGR_Fld_Destroy(self.c_obj as *mut c_void) };
    }
}

impl MajorObject for FieldDefn {
    unsafe fn gdal_object_ptr(&self) -> *const c_void {
        self.c_obj
    }
}

impl FieldDefn {
    pub fn new(name: &str, field_type: ogr_enums::OGRFieldType) -> Result<FieldDefn> {
        let c_str = CString::new(name)?;
        let c_obj = unsafe { ogr::OGR_Fld_Create(c_str.as_ptr(), field_type) };
        if c_obj.is_null() {
            return Err(_last_null_pointer_err("OGR_Fld_Create").into());
        };
        Ok(FieldDefn { c_obj: c_obj})
    }
    pub fn set_width(&self, width: i32) {
        unsafe {ogr:: OGR_Fld_SetWidth(self.c_obj as *mut c_void, width as c_int) };
    }
    pub fn set_precision(&self, precision: i32) {
        unsafe {ogr:: OGR_Fld_SetPrecision(self.c_obj as *mut c_void, precision as c_int) };
    }
    pub fn add_to_layer(&self, layer: &Layer) -> Result<()> {
        let rv = unsafe { ogr::OGR_L_CreateField(layer.gdal_object_ptr(), self.c_obj, 1) };
        if rv != ogr_enums::OGRErr::OGRERR_NONE {
            return Err(ErrorKind::OgrError(rv, "OGR_L_CreateFeature").into());
        }
        Ok(())
    }
}
