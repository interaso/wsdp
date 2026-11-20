# Official protocol and registry provenance

Retrieved on **2026-11-20** from official ČÚZK sources. Builds never download these inputs. Updates require an explicit review, regenerated checked-in artifacts, tests, and replacement hashes in this file.

## WSDP 3.1 and NTR

| Artifact | Official source | Version/update | SHA-256 |
| --- | --- | --- | --- |
| `WSDP_10_4_2026.zip` | [WSDP 3.1 documentation](https://cuzk.gov.cz/Katastr-nemovitosti/Poskytovani-udaju-z-KN/Dalkovy-pristup/Webove-sluzby-dalkoveho-pristupu/WSDP_10_4_2026.aspx) | Guide effective 2026-05-04 | `b2ebd76b275c47f96f55fd3aff45eb91e3b324ceadd603556d2d5e1af2336e7c` |
| `NTR_20261006.zip` | [NTR documentation](https://cuzk.gov.cz/Katastr-nemovitosti/Poskytovani-udaju-z-KN/Dalkovy-pristup/Webove-sluzby-dalkoveho-pristupu/NTR_20261006.aspx) | 2026-10-06 | `23811d2d11d330f6465d42624b3b0735bd70a4f8c2e62a8ab6781e601232e51e` |
| `ukazky.zip` | [Official examples](https://cuzk.gov.cz/Katastr-nemovitosti/Poskytovani-udaju-z-KN/Dalkovy-pristup/Webove-sluzby-dalkoveho-pristupu/ukazky.aspx) | Examples date from 2022 | `b298ab533952329dba6157a5c8d568aa07f6add3f21eb355a402945f729c7ef6` |

The bundled WSDL/XSD files are the primary wire contract. Trial behavior resolves runtime-enabled versions and discrepancies; it never silently overrides the XSD.

Nine anonymized XML documents from `ukazky.zip` are checked in under `tests/fixtures/legacy/` and parsed in full by offline tests. The archive hash above authenticates their source. The official unit example omits the XSD-required `PODLAHOVA_PLOCHA`; the unit parser has a narrow compatibility exception for this one missing field, with no relaxation of other required fields. Synthetic tests cover roots and edge cases absent from the official bundle, including NTR LV, proceeding information, account statements, and proceeding/monthly prices.

## Legal-relation registries

The [registry index](https://cuzk.gov.cz/Katastr-nemovitosti/Poskytovani-udaju-z-KN/Ciselniky-ISKN/Ciselniky-pravnich-vztahu-a-listin.aspx#SC_T_PRAVNICH_VZT) reported both registries as updated on **2026-11-20**. Checked-in CSVs under `registry/` are semicolon-delimited Windows-1250 snapshots with LF endings. Reconfirm encoding and shape at each refresh.

| Artifact | Official download | Snapshot | SHA-256 |
| --- | --- | --- | --- |
| `SC_T_PRAVNICH_VZT.zip` | [legal-relation ZIP](https://services.cuzk.cz/sestavy/cis/SC_T_PRAVNICH_VZT.zip) | 317 unique `KOD` rows through 320, including historical rows | `14b981dc4b0bf00ff6f2b695a1267484b390a1c977810693b1c44120b748066c` |
| `SC_T_PRAVNICH_VZT.csv` | Same archive | Code 320 valid from 2026-11-18 | `31868ccea32fc19c5578e3c035b1c7fd17cd8263bfa25139dcdda45867f43c7a` |
| `SC_T_PREDMETU_R.zip` | [proceeding-subject ZIP](https://services.cuzk.cz/sestavy/cis/SC_T_PREDMETU_R.zip) | 96 unique `KOD` rows | `90cbba441c0ff81c13294184c36ab4ea9f5529bc11db4edeb0d9c80b3ea8b486` |
| `SC_T_PREDMETU_R.csv` | Same archive | Referenced by `SC_T_PRAVNICH_VZT.TPR_KOD` | `b6d420ae3fb37ebe10ca5043adc60aa44d0f773f4e9cfb585909e4fde4f9de6f` |

## Trial

The [official trial page](https://cuzk.gov.cz/Katastr-nemovitosti/Poskytovani-udaju-z-KN/Dalkovy-pristup/Webove-sluzby-DP-na-zkousku.aspx), updated 2026-10-23, documents the endpoint, shared account types, sample data, and limitations. Public trial credentials are recorded in `PLAN.md`; live tests read them from the environment and never use production implicitly.

## Monthly price data

The checked-in `schemas/cenoveUdaje/1.0/cenoveUdaje.xsd` is the official [CenoveUdaje 1.0 schema](https://services.cuzk.gov.cz/xsd/cenoveUdaje/1.0/cenoveUdaje.xsd), retrieved 2026-11-20 with SHA-256 `a010e3f53d61ebe450e654af0d202fae952fbdd7fc22b8cc91a7ef593d5c59ff`. Its expanded root is `{http://services.cuzk.cz/xsd/cenoveUdaje/1.0}CenoveUdaje`. The schema imports GML 3.2 and GMD; the parser handles the used `PointPropertyType` structure and preserves association/ownership attributes without downloading external schemas.

Although GML prose recommends `gml:id` on objects, `AbstractGMLType` does not declare the attribute `use="required"`. Monthly feature/point IDs are therefore optional in the public model, and trial data exercises their omission.
