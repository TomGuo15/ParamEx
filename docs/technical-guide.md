# Technical Guide

This guide is the written form of the in-app **Technical guide** (the **?**
button in the top bar). It states exactly which files ParamEx accepts, what it
computes from them, and which columns it writes. The equations here are the
ones the code implements; when a fit is rejected, the guide says why.

Both workspaces plot the current magnitude `|I_D|`. Signed values are kept in
the data and written to the CSV exports unchanged.

## Transfer workspace

### Input files

| Rule | Value |
|---|---|
| Formats | `.csv`, `.tsv`, `.txt`, `.xls`, `.xlsx` |
| Columns | gate voltage and drain current, located by header label |
| Header scan | the header row may sit deep in an instrument preamble |
| Points | at least 12 measured points per file |

Header matching is case-insensitive, ignores units in parentheses, and accepts
these labels:

| Quantity | Accepted labels |
|---|---|
| Gate voltage | `Vg`, `Vgs`, `Gate`, `Gate Voltage` |
| Drain current | `Id`, `Ids`, `Drain`, `Drain Current` |

A file whose gate voltage reverses direction is a double sweep. ParamEx splits
it at the turning point into a forward and a backward branch and reports each
branch separately.

**Geometry.** Mobility needs `W` (µm), `L` (µm), and `C_ox` (nF/cm²). Every
file loads as `default` (1500 µm × 50 µm). **Apply W/L to All Files** writes
the workspace values to every file (`global`). Editing a file's own geometry
row sets that file to `manual`.

### Output curves

Output (I_D–V_D) files attach to a transfer file by name. The attachment key is
the transfer file stem; an output file matches when its stem is that key plus
one of the suffixes `-output-curve`, `-id-vd`, `-output`, or a trailing `o`
after a digit (for example `2-6o.xlsx` attaches to `2-6.xlsx`). Files whose
names contain `id-vd` or one of those suffixes are treated as output files when
loaded through **Load Folder**.

| Quantity | Accepted labels |
|---|---|
| Gate voltage | `Vg`, `Vgs`, `Gate`, `Gate Voltage` |
| Drain voltage | `Vd`, `Vds`, `Drain`, `Drain Voltage` |
| Drain current | `Id`, `Ids`, `Drain`, `Drain Current` |

Each distinct gate voltage in an output file is one *line*. The *family* row
aggregates every fitted line of the file.

### Fit mathematics

**Threshold voltage and saturation mobility** come from a straight line fitted
to the square root of the drain-current magnitude:

$$
\sqrt{|I_D|} = mV_G + b
$$

$$
V_{TH} = -\frac{b}{m}, \qquad
\mu_{sat} = \frac{2m^2}{C_{ox}\,(W/L)}
$$

The fit window is chosen automatically. Candidate windows of increasing width
are scored on R²; the ladder 0.99 → 0.97 → 0.95 → 0.90 is walked and the first
threshold any candidate clears wins, with near-equal candidates resolving toward
the wider window. A user-selected window replaces the automatic one for that
file and sweep direction; **Reset to Auto** returns to the selector. A manual
window needs at least 5 points. The fit is rejected, and
`V_TH` and `µ_sat` are left blank, when the window holds too few points, when the
slope is not finite or is effectively zero, or when the R² falls below the gate.
The slope, intercept, R², and point count of a rejected fit stay visible so the
weak fit can be inspected.

**Subthreshold swing** is the inverse slope of log current against gate
voltage. Samples within half a decade of the noise floor (the median of the
lowest quarter of the log-current samples) are masked out, then the steepest
5-point window that spans at least 0.3 decades with R² ≥ 0.9 is taken; the
span and R² gates relax in two steps before a global search over longer
windows runs as the last resort.

$$
\log_{10}|I_D| = sV_G + c, \qquad
SS = \left|\frac{1000}{s}\right|\ \mathrm{mV/dec}
$$

**On/off ratio** uses the largest and the smallest positive current magnitude
in the sweep:

$$
I_{on} = \max |I_D|, \qquad
I_{off} = \min_{|I_D|>0}|I_D|, \qquad
\mathrm{on/off} = \frac{I_{on}}{I_{off}}
$$

**Hysteresis** is defined only for a double sweep with at least 12 points in
each branch. Both branches are resampled onto a shared log-current grid and the
gate-voltage difference is taken at every level:

$$
\Delta V_{TH,\,hyst} = \mathrm{median}\left[V_G^{bwd}(\log I) - V_G^{fwd}(\log I)\right]
$$

**Output curves.** Every gate line is fitted as a straight line in drain
voltage:

$$
I_D = g_{ds}V_D + I_0
$$

$$
r_o = \frac{1}{|g_{ds}|}, \qquad
V_A = \frac{I_0}{g_{ds}}, \qquad
\lambda = \frac{1}{|V_A|}
$$

Without a user window the fit uses the two distinct samples at the largest
`|V_D|`, which is the flattest part of a saturated line; a user window replaces
it and must hold at least two distinct drain voltages. `I_{dsat}` is the current
at the largest `|V_D|` inside the window. The family row reports the largest
`I_{dsat}`, the median `g_{ds}`, R², and gate voltage over the fitted lines, and
the median Early voltage. The Early voltage is left blank when the lines
disagree in sign or spread more than tenfold in magnitude, because a single
number would then misrepresent the family. Its status is `partial` when some
lines did not fit and `unavailable` when none did.

### Exports

`paramex_report.csv` holds two sections, `Forward Results` and `Backward
Results`. Each section has a title row, the header below, one row per file, and
a closing `Overall` row with `mean ± std` for every metric. Single-sweep files
appear in the forward section only.

| Column | Meaning |
|---|---|
| `File` | transfer file name |
| `W (µm)`, `L (µm)`, `W/L` | geometry used for this file |
| `Geometry` | `default`, `global`, or `manual` |
| `VTH (V)` | threshold voltage |
| `mu_sat (cm^2 V^-1 s^-1)` | saturation mobility |
| `SS (mV dec^-1)` | subthreshold swing |
| `Ion`, `Ioff`, `Ion/Ioff` | on current, off current, ratio |
| `DeltaVTH,hyst (V)` | hysteresis (double sweeps only) |
| `Status`, `Message` | extraction outcome and explanation |

`paramex_output_report.csv` has one family row per attached output file
followed by its lines in ascending gate voltage:

```text
device,output_file,fit,status,Vg,Idsat,gds,ro,Early voltage,lambda,Vds fit min,Vds fit max,R2
```

`fit` is `Family` or `Line`; `status` is `ok`, `partial`, or `unavailable`.
Non-finite values are written as empty cells.

## TLM workspace

### Input files

The dataset root is a folder. The first level below it names the process
group; the second level is the channel length in micrometers; the workbooks
inside are the devices at that length.

```text
root/
  group/
    <length_um>/
      device.xlsx
```

Several groups may sit under the root, and several length folders under a
group. The length folder name is the channel length in µm (`50`, `80.5`).

| Rule | Value |
|---|---|
| Formats | `.xlsx` |
| Length folder | `<length_um>`, a number read as µm |
| Data sheet | a sheet named `List(*)` with columns `vg`, `abs_id`, `abs_is` |
| Bias sheet | a sheet named `Setup(*)` with a `Measurement.Bias.Source` row; the drain bias is the value under the `vd` channel named by `Channel.VName`, or the third value when channels are unnamed |
| Fallback V_D | used for every workbook without a `Setup(*)` sheet |

Currents in the `List(*)` sheet are read as magnitudes, so signed exports load
unchanged. The fallback drain voltage must be finite and non-zero; it is read
when the folder is loaded, so changing it afterwards requires a reload. Each
status row records whether its drain bias came from the `Setup(*)` sheet or
the fallback.

Every workbook the scan finds gets a status row, including workbooks that
failed to parse or folders that could not be listed. Those rows stay in the
status table and export so the failure is never silent.

### Fit mathematics

**Current at the selected gate voltage.** For each device the sample closest to
the selected `V_G` is taken. The channel current is the smaller of the drain and
source magnitudes at that sample, which discards gate-leakage contributions. At
each length the device with the highest channel current is the *primary*
selection; the median across devices is a *diagnostic* alternative.

$$
I_{ch,d} = \min(|I_{D,d}|, |I_{S,d}|), \qquad
I_L = \max_d I_{ch,d}, \qquad
R_{tot}(L) = \frac{|V_D|}{I_L}
$$

**Length fit.** Total resistance is fitted against channel length by ordinary
least squares. The intercept is twice the contact resistance:

$$
R_{tot}(L) = mL + b = mL + 2R_c, \qquad
R_c/\mathrm{contact} = \frac{b}{2}, \qquad
R^2 = 1 - \frac{SS_{res}}{SS_{tot}}
$$

A fit needs at least two distinct lengths; R² needs three. The slope `m` is in
Ω/µm and is *not* the sheet resistance (that would require dividing by the
channel width). ParamEx labels the fitted intercept `intercept (2R_c)` and the
derived value `R_c/contact` so the two are never confused.

The **sweep** repeats the length fit at every measured gate voltage so the
contact resistance can be read as a function of `V_G`.

### Exports

`paramex_tlm_result.csv` has one row per group at the selected gate voltage;
`paramex_tlm_sweep.csv` has one row per group and gate voltage. Both share the
same columns:

| Column | Meaning |
|---|---|
| `group` | process group folder |
| `selected_vg` | gate voltage of the fit |
| `Rcontact_script_ohm` | fitted intercept, `2R_c`, primary selection |
| `Rc_per_contact_ohm` | `R_c` per contact, primary selection |
| `slope_ohm_per_um` | fitted slope, primary selection |
| `r_squared` | coefficient of determination, primary selection |
| `Rcontact_median_ohm`, `Rc_per_contact_median_ohm`, `slope_median_ohm_per_um`, `r_squared_median` | the same four values for the median selection |
| `valid_lengths` | number of lengths that contributed to the fit |
| `warnings` | `;`-separated fit warnings |

Blank cells are values the fit could not produce.
