/// Complete legacy `SeznamDefinicnichBoduParcel` report.
#[derive(Clone, Eq, PartialEq)]
#[non_exhaustive]
pub struct DefinitionPointsReport {
    /// Every definition point in server order.
    pub points: Vec<DefinitionPoint>,
    /// Lexical validity timestamp (`PLATNOST`).
    pub valid_at: String,
    /// Report creator (`VYHOTOVIL`).
    pub created_by: String,
    /// Free-access notice (`BEZPL_PRISTUP`).
    pub free_access: String,
    /// Lexical creation timestamp (`VYHOTOVENO`).
    pub created_at: String,
}

/// One definition-point row. All fields intentionally remain strings because the
/// official schema declares strings rather than IDs or decimal coordinates.
#[derive(Clone, Eq, PartialEq)]
#[non_exhaustive]
pub struct DefinitionPoint {
    /// S-JTSK Y coordinate lexical value.
    pub coordinate_y: String,
    /// S-JTSK X coordinate lexical value.
    pub coordinate_x: String,
    /// Cadastral-area code lexical value.
    pub cadastral_area_code: String,
    /// Parcel ID lexical value.
    pub parcel_id: String,
    /// Municipality-part code lexical value.
    pub municipality_part_code: String,
    /// Building ID lexical value.
    pub building_id: String,
}
