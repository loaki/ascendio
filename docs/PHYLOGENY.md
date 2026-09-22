# Phylogeny data, sources, and the simplifications we made

The tree lives in [`src/tree.rs`](../src/tree.rs) as a flat table of 68 taxa.
Each row is `(common name, clade, origin in Ma, parent, colour group)`.

This document records where the topology and the dates come from, and — more
importantly — where we knowingly departed from the literature for the sake of a
playable tree.

## Backbone sources

| Region of the tree | Primary source |
|---|---|
| Metazoa backbone, phylum-level sampling | Laumer et al. 2019, *Proc. R. Soc. B* 286:20190831 — [10.1098/rspb.2019.0831](https://doi.org/10.1098/rspb.2019.0831) |
| Animal phylogeny overview and its implications | Dunn, Giribet, Edgecombe & Hejnol 2014, *Annu. Rev. Ecol. Evol. Syst.* 45:371–395 — [10.1146/annurev-ecolsys-120213-091627](https://doi.org/10.1146/annurev-ecolsys-120213-091627) |
| Broad phylogenomic sampling of Metazoa | Dunn et al. 2008, *Nature* 452:745–749 — [10.1038/nature06614](https://doi.org/10.1038/nature06614) |
| Placement of Xenacoelomorpha | Cannon et al. 2016, *Nature* 530:89–93 — [10.1038/nature16520](https://doi.org/10.1038/nature16520) |
| Placozoa / Cnidaria | Laumer et al. 2018, *eLife* 7:e36278 — [10.7554/eLife.36278](https://doi.org/10.7554/eLife.36278) |
| Insect timing and pattern | Misof et al. 2014, *Science* 346:763–767 — [10.1126/science.1257570](https://doi.org/10.1126/science.1257570) |
| Jawed vertebrate timetree | Irisarri et al. 2017, *Nat. Ecol. Evol.* 1:1370–1378 — [10.1038/s41559-017-0240-5](https://doi.org/10.1038/s41559-017-0240-5) |
| Mammal tree | Upham, Esselstyn & Jetz 2019, *PLoS Biol.* 17:e3000494 — [10.1371/journal.pbio.3000494](https://doi.org/10.1371/journal.pbio.3000494) |
| Bird phylogeny | Prum et al. 2015, *Nature* 526:569–573 — [10.1038/nature15697](https://doi.org/10.1038/nature15697) |
| Divergence dates | TimeTree 5 — Kumar et al. 2022, *Mol. Biol. Evol.* 39:msac174 — [10.1093/molbev/msac174](https://doi.org/10.1093/molbev/msac174) |

## The root

The game starts at the **Urmetazoan**: the last common ancestor of all animals,
placed at ~800 Ma. Molecular-clock estimates for this node range roughly
650–850 Ma depending on calibration, so treat every date in the table as a
round number for flavour, not a citation.

## Simplifications we made on purpose

These are departures from the published topology. They are deliberate, and each
one is cheap to undo — the table is the only thing that would change.

1. **The base of Metazoa is a polytomy.** Porifera, Ctenophora, Placozoa,
   Cnidaria and Bilateria all hang directly off the root. This sidesteps the
   unresolved *Ctenophora-sister* vs. *Porifera-sister* debate — compare
   Laumer et al. 2019 (Porifera-sister) with Schultz et al. 2023, *Nature*
   618:110–117 ([10.1038/s41586-023-05936-6](https://doi.org/10.1038/s41586-023-05936-6),
   Ctenophora-sister from gene linkage data). Picking a side would make one
   branch of a game tree feel arbitrarily privileged; a polytomy does not.

2. **Nephrozoa is collapsed.** Cannon et al. 2016 place Xenacoelomorpha as
   sister to Nephrozoa (Protostomia + Deuterostomia). We hang all three off
   Bilateria directly, dropping the intermediate Nephrozoa node.

3. **Clade names stand in for animals.** "Elephant" is the visible label for
   Afrotheria, "Whale" for Cetartiodactyla, "Wolf" for Carnivora. The player
   sees a recognisable animal; the subtitle shows the real clade. A future pass
   should split these into a proper genus-level tip below the clade node.

4. **Breadth is heavily pruned.** 68 nodes stand in for well over a million
   described animal species. Whole phyla with no obvious mascot (Bryozoa,
   Nemertea, Onychophora, Priapulida…) are absent.

5. **Ranks are mixed.** The table freely interleaves phyla, classes, orders and
   one species (*Homo sapiens*). Depth in this tree is a game progression axis,
   not a taxonomic rank.

## Invariants the tests enforce

`cargo test` checks two properties of the table, so a bad edit fails loudly:

- every taxon is reachable from the root (no orphans, no cycles);
- no child is dated older than its parent.
