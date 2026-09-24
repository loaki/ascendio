# Phylogeny data, sources, and the simplifications we made

The tree lives in [`src/tree.rs`](../src/tree.rs) as a flat table of 151 taxa. The last 83 were added later (29, then the
54 of the 151 expansion) and are appended, so older saves keep their indices.
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

3. **Clade names used to stand in for animals.** "Elephant", "Whale",
   "Wolf" and "Kangaroo" were once the labels of Afrotheria, Cetartiodactyla,
   Carnivora and Marsupialia. The audit renamed those nodes (Afrothere,
   Cetartiodactyl, Carnivoran, Marsupial) and gave each animal a real tip
   below them. One is left: "Lion" is still Felidae, so Tiger, Cat and
   Sabre-tooth hang from it rather than from a separate cat-family node.

4. **Breadth is heavily pruned.** 151 nodes stand in for well over a million
   described animal species. Some phyla with no obvious mascot (Rotifera,
   Priapulida, Chaetognatha…) are still absent.

5. **Ranks are mixed.** The table freely interleaves phyla, classes, orders and
   one species (*Homo sapiens*). Depth in this tree is a game progression axis,
   not a taxonomic rank.

6. **More collapsed nodes (the 151 expansion).** Moss Animal hangs off
   Spiralia (really inside Lophotrochozoa); Fly, True Bug and Termite off
   Insecta (really Holometabola, Paraneoptera and Dictyoptera); Mite off
   Chelicerata (Acari may not even be one group); Sloth off Placentalia
   (really Xenarthra); Lungfish off Sarcopterygii (really sister to
   tetrapods, in Rhipidistia, but 410 Ma is older than our Tetrapod node);
   Carp off Actinopterygii; Chicken, Songbird, Dodo and the other bird tips
   straight off Aves. Plesiosaurs and ichthyosaurs hang off Sauropsida
   because their place within Diapsida is unresolved, and Triceratops off
   Dinosauria (no Ornithischia/Saurischia split). Dickinsonia hangs off the
   root: it is an animal, but where it sits within Metazoa is debated.

7. **Nodes inserted after the fact.** Synapsid, Archosaur, Theropod,
   Hominin, Perissodactyl, Spiny-rayed Fish and Wasp were added so the tree
   tells its main stories (the mammal line, crocodiles as birds' kin,
   dinosaurs to birds, the human line). Their rows are appended and the
   existing children re-parented to them, so a node's row can come after its
   children's; `Phylogeny::load` computes depth from the root down.

## The 151 expansion

54 taxa were added after the first 97, again appended so saves keep their
indices. Each date is a round number: a crown age, an earliest fossil or a
split from the sister group, whichever the source gives best, and always no
older than its parent in the table. TimeTree 5 filled in where a paper gave
the topology but not the date.

A three-way audit then checked the list for missing groups (by species
count), for animals players expect (favourite-animal surveys such as
Albert et al. 2018, *PLOS ONE* —
[10.1371/journal.pone.0199149](https://doi.org/10.1371/journal.pone.0199149))
and for the milestones of animal evolution. It added 32 taxa, among them
the four real animals behind the renamed clade nodes, and cut 32
near-duplicates and obscure tips to stay at 151. The cut taxa:
Sea Anemone, Hydra, Ribbon Worm, Lamp Shell, Chiton, Sea Slug, Leech, Velvet Worm, Sea Spider, Barnacle, Mantis Shrimp, Grasshopper, Praying Mantis, Firefly, Sea Cucumber, Sea Lily, Acorn Worm, Lancelet, Hagfish, Sturgeon, Eel, Pufferfish, Hammerhead, Toad, Axolotl, Chameleon, Komodo Dragon, Stegosaurus, Hummingbird, Echidna, Manatee, Orangutan.

| Taxon | Clade | Parent | Ma | Source |
|---|---|---|---|---|
| Nautilus | *Nautilida* | Octopus | 415 | Tanner et al. 2017, Proc. R. Soc. B — [10.1098/rspb.2016.2818](https://doi.org/10.1098/rspb.2016.2818) |
| Fly | *Diptera* | Insect | 260 | Wiegmann et al. 2011, PNAS — [10.1073/pnas.1012675108](https://doi.org/10.1073/pnas.1012675108) |
| Clownfish | *Amphiprion* | Spiny-rayed Fish | 13 | Litsios et al. 2012, BMC Evol. Biol. — [10.1186/1471-2148-12-212](https://doi.org/10.1186/1471-2148-12-212) |
| Lungfish | *Dipnoi* | Lobe-finned Fish | 410 | Irisarri et al. 2017, Nat. Ecol. Evol. — [10.1038/s41559-017-0240-5](https://doi.org/10.1038/s41559-017-0240-5) |
| Pig | *Suidae* | Cetartiodactyl | 37 | Gongora et al. 2011, Zool. Scr. — [10.1111/j.1463-6409.2011.00480.x](https://doi.org/10.1111/j.1463-6409.2011.00480.x) |
| Tuatara | *Rhynchocephalia* | Lizard | 230 | Gemmell et al. 2020, Nature — [10.1038/s41586-020-2561-9](https://doi.org/10.1038/s41586-020-2561-9) |
| Sea Turtle | *Chelonioidea* | Turtle | 120 | Cadena & Parham 2015, PaleoBios — [10.5070/P9321028615](https://doi.org/10.5070/P9321028615) |
| Plesiosaur | *Plesiosauria* | Reptile | 205 | Wintrich et al. 2017, Sci. Adv. — [10.1126/sciadv.1701144](https://doi.org/10.1126/sciadv.1701144) |
| Ichthyosaur | *Ichthyosauria* | Reptile | 250 | Kear et al. 2023, Curr. Biol. — [10.1016/j.cub.2022.12.053](https://doi.org/10.1016/j.cub.2022.12.053) |
| Triceratops | *Triceratops* | Dinosaur | 68 | Longrich & Field 2012, PLoS ONE — [10.1371/journal.pone.0032623](https://doi.org/10.1371/journal.pone.0032623) |
| Velociraptor | *Velociraptor* | Theropod | 75 | Turner et al. 2007, Science — [10.1126/science.1145076](https://doi.org/10.1126/science.1145076) |
| Archaeopteryx | *Archaeopteryx* | Theropod | 150 | Foth et al. 2014, Nature — [10.1038/nature13467](https://doi.org/10.1038/nature13467) |
| Ostrich | *Struthio* | Bird | 80 | Yonezawa et al. 2017, Curr. Biol. — [10.1016/j.cub.2016.10.029](https://doi.org/10.1016/j.cub.2016.10.029) |
| Parrot | *Psittaciformes* | Bird | 55 | Prum et al. 2015, Nature — [10.1038/nature15697](https://doi.org/10.1038/nature15697) |
| Owl | *Strigiformes* | Bird | 60 | Prum et al. 2015, Nature — [10.1038/nature15697](https://doi.org/10.1038/nature15697) |
| Koala | *Phascolarctos* | Marsupial | 35 | Johnson et al. 2018, Nat. Genet. — [10.1038/s41588-018-0153-5](https://doi.org/10.1038/s41588-018-0153-5) |
| Sloth | *Folivora* | Placental | 35 | Delsuc et al. 2019, Curr. Biol. — [10.1016/j.cub.2019.05.043](https://doi.org/10.1016/j.cub.2019.05.043) |
| Giraffe | *Giraffidae* | Cetartiodactyl | 20 | Farré et al. 2019, GigaScience — [10.1093/gigascience/giz090](https://doi.org/10.1093/gigascience/giz090) |
| Hippo | *Hippopotamidae* | Cetartiodactyl | 16 | Boisserie, Lihoreau & Brunet 2005, PNAS — [10.1073/pnas.0409518102](https://doi.org/10.1073/pnas.0409518102) |
| Rhino | *Rhinocerotidae* | Perissodactyl | 16 | Liu et al. 2021, Cell — [10.1016/j.cell.2021.07.032](https://doi.org/10.1016/j.cell.2021.07.032) |
| Bear | *Ursidae* | Carnivoran | 20 | Krause et al. 2008, BMC Evol. Biol. — [10.1186/1471-2148-8-220](https://doi.org/10.1186/1471-2148-8-220) |
| Seal | *Pinnipedia* | Carnivoran | 25 | Higdon et al. 2007, BMC Evol. Biol. — [10.1186/1471-2148-7-216](https://doi.org/10.1186/1471-2148-7-216) |
| Synapsid | *Synapsida* | Amniote | 315 | TimeTree 5 — Kumar et al. 2022, Mol. Biol. Evol. — [10.1093/molbev/msac174](https://doi.org/10.1093/molbev/msac174) |
| Archosaur | *Archosauria* | Reptile | 250 | TimeTree 5 — Kumar et al. 2022, Mol. Biol. Evol. — [10.1093/molbev/msac174](https://doi.org/10.1093/molbev/msac174) |
| Theropod | *Theropoda* | Dinosaur | 230 | TimeTree 5 — Kumar et al. 2022, Mol. Biol. Evol. — [10.1093/molbev/msac174](https://doi.org/10.1093/molbev/msac174) |
| Hominin | *Hominini* | Ape | 7 | TimeTree 5 — Kumar et al. 2022, Mol. Biol. Evol. — [10.1093/molbev/msac174](https://doi.org/10.1093/molbev/msac174) |
| Perissodactyl | *Perissodactyla* | Placental | 60 | TimeTree 5 — Kumar et al. 2022, Mol. Biol. Evol. — [10.1093/molbev/msac174](https://doi.org/10.1093/molbev/msac174) |
| Spiny-rayed Fish | *Acanthomorpha* | Ray-finned Fish | 130 | Near et al. 2013, PNAS — [10.1073/pnas.1304661110](https://doi.org/10.1073/pnas.1304661110) |
| Elephant | *Elephantidae* | Afrothere | 7 | Rohland et al. 2007, PLoS Biol. — [10.1371/journal.pbio.0050207](https://doi.org/10.1371/journal.pbio.0050207) |
| Whale | *Cetacea* | Cetartiodactyl | 53 | McGowen et al. 2019, Syst. Biol. — [10.1093/sysbio/syz068](https://doi.org/10.1093/sysbio/syz068) |
| Wolf | *Canidae* | Carnivoran | 40 | TimeTree 5 — Kumar et al. 2022, Mol. Biol. Evol. — [10.1093/molbev/msac174](https://doi.org/10.1093/molbev/msac174) |
| Dog | *Canis familiaris* | Wolf | 0.02 | Bergström et al. 2020, Science — [10.1126/science.aba9572](https://doi.org/10.1126/science.aba9572) |
| Kangaroo | *Macropodidae* | Marsupial | 15 | Mitchell et al. 2014, Mol. Biol. Evol. — [10.1093/molbev/msu176](https://doi.org/10.1093/molbev/msu176) |
| Tiger | *Panthera tigris* | Lion | 3 | Johnson et al. 2006, Science — [10.1126/science.1122277](https://doi.org/10.1126/science.1122277) |
| Cat | *Felis* | Lion | 6 | Johnson et al. 2006, Science — [10.1126/science.1122277](https://doi.org/10.1126/science.1122277) |
| Sabre-tooth | *Smilodon* | Lion | 2.5 | Paijmans et al. 2017, Curr. Biol. — [10.1016/j.cub.2017.09.033](https://doi.org/10.1016/j.cub.2017.09.033) |
| Giant Panda | *Ailuropoda* | Bear | 19 | Krause et al. 2008, BMC Evol. Biol. — [10.1186/1471-2148-8-220](https://doi.org/10.1186/1471-2148-8-220) |
| Cow | *Bovidae* | Cetartiodactyl | 18 | Chen et al. 2019, Science — [10.1126/science.aav6202](https://doi.org/10.1126/science.aav6202) |
| Chicken | *Galloanserae* | Bird | 67 | Field et al. 2020, Nature — [10.1038/s41586-020-2096-0](https://doi.org/10.1038/s41586-020-2096-0) |
| Songbird | *Passeriformes* | Bird | 47 | Oliveros et al. 2019, PNAS — [10.1073/pnas.1813206116](https://doi.org/10.1073/pnas.1813206116) |
| Dodo | *Raphus* | Bird | 25 | Shapiro et al. 2002, Science — [10.1126/science.295.5560.1683](https://doi.org/10.1126/science.295.5560.1683) |
| True Bug | *Hemiptera* | Insect | 310 | Johnson et al. 2018, PNAS — [10.1073/pnas.1815820115](https://doi.org/10.1073/pnas.1815820115) |
| Wasp | *Hymenoptera* | Insect | 280 | Peters et al. 2017, Curr. Biol. — [10.1016/j.cub.2017.01.027](https://doi.org/10.1016/j.cub.2017.01.027) |
| Mite | *Acari* | Chelicerate | 410 | Pepato et al. 2022, Mol. Phylogenet. Evol. — [10.1016/j.ympev.2022.107626](https://doi.org/10.1016/j.ympev.2022.107626) |
| Termite | *Termitoidae* | Insect | 140 | Bucek et al. 2019, Curr. Biol. — [10.1016/j.cub.2019.08.076](https://doi.org/10.1016/j.cub.2019.08.076) |
| Moss Animal | *Bryozoa* | Spiralian | 485 | Zhang et al. 2021, Nature — [10.1038/s41586-021-04033-w](https://doi.org/10.1038/s41586-021-04033-w) |
| Brittle Star | *Ophiuroidea* | Echinoderm | 270 | O'Hara et al. 2014, Curr. Biol. — [10.1016/j.cub.2014.06.060](https://doi.org/10.1016/j.cub.2014.06.060) |
| Carp | *Ostariophysi* | Ray-finned Fish | 150 | Hughes et al. 2018, PNAS — [10.1073/pnas.1719358115](https://doi.org/10.1073/pnas.1719358115) |
| Dickinsonia | *Dickinsonia* | Urmetazoan | 558 | Bobrovskiy et al. 2018, Science — [10.1126/science.aat7228](https://doi.org/10.1126/science.aat7228) |
| Dimetrodon | *Dimetrodon* | Synapsid | 295 | Brink & Reisz 2014, Nat. Commun. — [10.1038/ncomms4269](https://doi.org/10.1038/ncomms4269) |
| Lucy | *Australopithecus* | Hominin | 3.9 | Johanson & White 1979, Science — [10.1126/science.104384](https://doi.org/10.1126/science.104384) |
| Neanderthal | *Homo neanderthalensis* | Hominin | 0.43 | Meyer et al. 2016, Nature — [10.1038/nature17405](https://doi.org/10.1038/nature17405) |
| Pakicetus | *Pakicetus* | Whale | 50 | Thewissen et al. 2001, Nature — [10.1038/35095005](https://doi.org/10.1038/35095005) |
| Acanthostega | *Acanthostega* | Tetrapod | 365 | Coates & Clack 1990, Nature — [10.1038/347066a0](https://doi.org/10.1038/347066a0) |

Softer dates, worth revisiting: Wolf and the inserted nodes (TimeTree 5
round numbers), Carp (the Ostariophysi age is an estimate), Kangaroo (15 Ma
for Macropodidae), Moss Animal (485 Ma is the first undisputed fossil;
Zhang et al. 2021 argue for 530) and Whale (53 Ma is the split from hippos,
the crown group is about 36). Dog and Neanderthal are species, dated from
domestication and from their split from our lineage.

## Invariants the tests enforce

`cargo test` checks two properties of the table, so a bad edit fails loudly:

- every taxon is reachable from the root (no orphans, no cycles);
- no child is dated older than its parent.
