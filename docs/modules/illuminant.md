# Lighting consistency (`illuminant`) — EXPERIMENTAL

**Shows:** per-block illuminant colour estimated with the grey-edge hypothesis (average gradient magnitude per channel, saturated/dark pixels excluded), plus the distance of each block from the image median.

**No automatic finding** is produced: illuminant estimation is easily biased by coloured surfaces, and the method only hints at objects photographed under different light. Use it as a visual aid next to the original.

**Not implemented:** 3-D lighting direction from occluding contours (Johnson & Farid) — it needs manual contour annotation.

**References.** J. van de Weijer, T. Gevers, A. Gijsenij, "Edge-based color constancy", IEEE TIP 2007; C. Riess, E. Angelopoulou, "Scene illumination as an indicator of image manipulation", IH 2010; M. K. Johnson, H. Farid, "Exposing digital forgeries by detecting inconsistencies in lighting", ACM MM&Sec 2005.
