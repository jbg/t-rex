//
// Copyright (c) Pirmin Kalberer. All rights reserved.
// Licensed under the MIT License. See LICENSE file in the project root for full license information.
//

use datasource::DatasourceInput;
use gdal;
use gdal::vector::{Dataset, FieldValue};
use core::feature::{Feature, FeatureAttr, FeatureAttrValType};
use core::geom::{self, GeometryType};
use core::grid::Extent;
use core::grid::Grid;
use core::layer::Layer;
use std::path::Path;


pub struct GdalDatasource {
    pub path: String,
}

impl GdalDatasource {
    pub fn new(path: &str) -> GdalDatasource {
        GdalDatasource { path: path.to_string() }
    }
}

struct VectorFeature<'a> {
    layer: &'a Layer,
    fields_defn: &'a Vec<gdal::vector::Field<'a>>,
    feature: &'a gdal::vector::Feature<'a>,
}


impl<'a> Feature for VectorFeature<'a> {
    fn fid(&self) -> Option<u64> {
        None // TODO
    }
    fn attributes(&self) -> Vec<FeatureAttr> {
        let mut attrs = Vec::new();
        for (_i, field) in self.fields_defn.into_iter().enumerate() {
            let field_value = self.feature.field(&field.name()); //TODO: get by index
            let val = match field_value {
                Ok(FieldValue::StringValue(v)) => Some(FeatureAttrValType::String(v)),
                Ok(FieldValue::IntegerValue(v)) => Some(FeatureAttrValType::Int(v as i64)),
                Ok(FieldValue::RealValue(v)) => Some(FeatureAttrValType::Double(v)),
                Err(err) => {
                    warn!("Layer '{}' - skipping field '{}': {}",
                          self.layer.name,
                          field.name(),
                          err);
                    None
                }
            };
            // match field.field_type {
            //    OGRFieldType::OFTString => {
            if let Some(val) = val {
                let fattr = FeatureAttr {
                    key: field.name(),
                    value: val,
                };
                attrs.push(fattr);
            };
        }
        attrs
    }
    fn geometry(&self) -> Result<GeometryType, String> {
        let _ogrgeom = self.feature.geometry();
        Ok(GeometryType::Point(geom::Point::new(960000.0, 6002729.0, Some(3857)))) //TODO
    }
}

impl DatasourceInput for GdalDatasource {
    fn retrieve_features<F>(&self,
                            layer: &Layer,
                            _extent: &Extent,
                            _zoom: u8,
                            _grid: &Grid,
                            mut read: F)
        where F: FnMut(&Feature)
    {
        let mut dataset = Dataset::open(Path::new(&self.path)).unwrap();
        let ogr_layer = dataset
            .layer_by_name(layer.table_name.as_ref().unwrap())
            .unwrap();
        let fields_defn = ogr_layer.defn().fields().collect::<Vec<_>>();
        for feature in ogr_layer.features() {
            let feat = VectorFeature {
                layer: layer,
                fields_defn: &fields_defn,
                feature: &feature,
            };
            read(&feat);
        }
    }
}



#[test]
fn test_gdal_api() {
    use std::path::Path;
    use gdal::vector::Dataset;

    let mut dataset = Dataset::open(Path::new("natural_earth.gpkg")).unwrap();
    let layer = dataset.layer_by_name("ne_10m_populated_places").unwrap();
    let feature = layer.features().next().unwrap();
    let name_field = feature.field("NAME").unwrap();
    let geometry = feature.geometry();
    assert_eq!(name_field.to_string(),
               Some("Colonia del Sacramento".to_string()));
    assert_eq!(geometry.wkt().unwrap(),
               "POINT (-6438719.62282072 -4093437.71441017)".to_string());
}



#[test]
fn test_gdal_retrieve() {
    let mut layer = Layer::new("points");
    layer.table_name = Some(String::from("ne_10m_populated_places"));
    layer.geometry_field = Some(String::from("wkb_geometry"));
    layer.geometry_type = Some(String::from("POINT"));
    let grid = Grid::web_mercator();
    let extent = Extent {
        minx: 821850.9,
        miny: 5909499.5,
        maxx: 860986.7,
        maxy: 5948635.3,
    };

    let ds = GdalDatasource::new("natural_earth.gpkg");
    let mut reccnt = 0;
    ds.retrieve_features(&layer, &extent, 10, &grid, |feat| {
        assert_eq!("Ok(Point(Point { x: 960000, y: 6002729, srid: Some(3857) }))",
                   &*format!("{:?}", feat.geometry()));
        assert_eq!(3, feat.attributes().len());
        assert_eq!(feat.attributes()[0].key, "SCALERANK");
        assert_eq!(feat.attributes()[1].key, "NAME");
        assert_eq!(feat.attributes()[2].key, "POP_MAX");
        if reccnt == 0 {
            assert_eq!(feat.attributes()[0].value, FeatureAttrValType::Int(10));
            assert_eq!(feat.attributes()[1].value,
                       FeatureAttrValType::String("Colonia del Sacramento".to_string()));
            assert_eq!(feat.attributes()[2].value, FeatureAttrValType::Int(21714));
        }
        assert_eq!(None, feat.fid());
        reccnt += 1;
    });
    assert_eq!(14644, reccnt);
}