// extraire-casc : lecture seule d'une installation Blizzard CASC, via CascLib.
//
//   extraire-casc lister         <racine> [motif] [--sortie liste.txt]
//   extraire-casc extraire       <racine> <motif> <dossier-sortie> [--plat]
//   extraire-casc extraire-liste <racine> <noms.txt> <dossier-sortie> [--plat]
//
// Le motif est un joker simple, insensible a la casse : `*` couvre n'importe quoi, `?` un
// caractere. Sans joker, il est traite comme un fragment, donc `*motif*`.
//
// `lister` ecrit « taille<TAB>nom » ; `extraire-liste` relit ce format, ou une simple liste de
// noms, ce qui evite de reparcourir les 780 000 entrees du stockage pour choisir dix fichiers.

#include <windows.h>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>
#include "CascLib.h"

// Abaisse la casse et ramene les separateurs a `/`, pour que motif et nom se comparent.
static std::string abaisse(const std::string & s)
{
    std::string r(s);
    for (size_t i = 0; i < r.size(); i++)
    {
        if (r[i] >= 'A' && r[i] <= 'Z') r[i] = (char)(r[i] + 32);
        if (r[i] == '\\') r[i] = '/';
    }
    return r;
}

static bool concorde(const char * motif, const char * texte)
{
    while (*motif)
    {
        if (*motif == '*')
        {
            motif++;
            if (*motif == 0) return true;
            for (const char * t = texte; *t; t++)
                if (concorde(motif, t)) return true;
            return false;
        }
        if (*texte == 0) return false;
        if (*motif != '?' && *motif != *texte) return false;
        motif++; texte++;
    }
    return *texte == 0;
}

static std::string motif_normalise(const std::string & brut)
{
    std::string m = abaisse(brut);
    if (m.find('*') == std::string::npos && m.find('?') == std::string::npos)
        m = "*" + m + "*";
    return m;
}

static void cree_arborescence(const std::string & chemin)
{
    for (size_t i = 0; i < chemin.size(); i++)
    {
        if (chemin[i] == '\\' || chemin[i] == '/')
            CreateDirectoryA(chemin.substr(0, i).c_str(), NULL);
    }
    CreateDirectoryA(chemin.c_str(), NULL);
}

static bool ecrit_fichier(HANDLE hStorage, const char * szNomCasc, const std::string & destination)
{
    HANDLE hFile = NULL;
    if (!CascOpenFile(hStorage, szNomCasc, 0, CASC_OPEN_BY_NAME, &hFile))
    {
        fprintf(stderr, "  ouverture refusee (%u) : %s\n", GetCascError(), szNomCasc);
        return false;
    }

    FILE * sortie = fopen(destination.c_str(), "wb");
    if (sortie == NULL)
    {
        fprintf(stderr, "  ecriture impossible : %s\n", destination.c_str());
        CascCloseFile(hFile);
        return false;
    }

    std::vector<BYTE> tampon(0x10000);
    bool ok = true;
    for (;;)
    {
        DWORD lu = 0;
        if (!CascReadFile(hFile, &tampon[0], (DWORD)tampon.size(), &lu))
        {
            DWORD err = GetCascError();
            if (err != ERROR_SUCCESS && err != ERROR_HANDLE_EOF)
            {
                fprintf(stderr, "  lecture interrompue (%u) : %s\n", err, szNomCasc);
                ok = false;
            }
            break;
        }
        if (lu == 0) break;
        fwrite(&tampon[0], 1, lu, sortie);
    }

    fclose(sortie);
    CascCloseFile(hFile);
    if (!ok) DeleteFileA(destination.c_str());
    return ok;
}

static HANDLE ouvre(const char * racine)
{
    HANDLE hStorage = NULL;
    if (!CascOpenStorage(racine, 0, &hStorage))
    {
        fprintf(stderr, "Ouverture du stockage impossible (%u) : %s\n", GetCascError(), racine);
        return NULL;
    }

    CASC_STORAGE_PRODUCT produit;
    memset(&produit, 0, sizeof(produit));
    if (CascGetStorageInfo(hStorage, CascStorageProduct, &produit, sizeof(produit), NULL))
        fprintf(stderr, "Stockage ouvert : produit %s, build %u\n", produit.szCodeName, produit.BuildNumber);

    return hStorage;
}

struct Entree
{
    std::string nom;
    ULONGLONG taille;
};

// Parcourt tout le stockage et rend les entrees qui concordent. L'enumeration est faite d'un
// bloc, avant toute ouverture de fichier : CascLib ne garantit pas les deux en meme temps.
static std::vector<Entree> recense(HANDLE hStorage, const std::string & motif, unsigned long * pTotal)
{
    std::vector<Entree> entrees;
    CASC_FIND_DATA trouve;
    HANDLE hFind = CascFindFirstFile(hStorage, "*", &trouve, NULL);
    unsigned long total = 0;

    if (hFind != NULL)
    {
        do
        {
            total++;
            if (concorde(motif.c_str(), abaisse(trouve.szFileName).c_str()))
            {
                Entree e;
                e.nom = trouve.szFileName;
                e.taille = trouve.FileSize;
                entrees.push_back(e);
            }
        }
        while (CascFindNextFile(hFind, &trouve));
        CascFindClose(hFind);
    }

    if (pTotal) *pTotal = total;
    return entrees;
}

// Extrait une liste de noms deja arretee. Rend le nombre de reussites.
static size_t extrait_tout(HANDLE hStorage, const std::vector<std::string> & noms,
                           const std::string & dossier, bool plat)
{
    size_t ok = 0;
    for (size_t i = 0; i < noms.size(); i++)
    {
        std::string relatif = noms[i];
        for (size_t j = 0; j < relatif.size(); j++) if (relatif[j] == '/') relatif[j] = '\\';

        if (plat)
        {
            size_t barre = relatif.find_last_of('\\');
            if (barre != std::string::npos) relatif = relatif.substr(barre + 1);
        }

        std::string destination = dossier + "\\" + relatif;
        size_t barre = destination.find_last_of('\\');
        if (barre != std::string::npos) cree_arborescence(destination.substr(0, barre));

        if (ecrit_fichier(hStorage, noms[i].c_str(), destination)) ok++;
    }
    return ok;
}

static int commande_lister(int argc, char ** argv)
{
    if (argc < 3) { fprintf(stderr, "usage: extraire-casc lister <racine> [motif] [--sortie liste.txt]\n"); return 2; }

    std::string motif = "*";
    std::string fichierSortie;
    for (int i = 3; i < argc; i++)
    {
        if (strcmp(argv[i], "--sortie") == 0 && i + 1 < argc) fichierSortie = argv[++i];
        else motif = motif_normalise(argv[i]);
    }

    HANDLE hStorage = ouvre(argv[2]);
    if (hStorage == NULL) return 1;

    unsigned long total = 0;
    std::vector<Entree> entrees = recense(hStorage, motif, &total);

    FILE * sortie = stdout;
    if (!fichierSortie.empty())
    {
        sortie = fopen(fichierSortie.c_str(), "wb");
        if (sortie == NULL) { fprintf(stderr, "ecriture impossible : %s\n", fichierSortie.c_str()); return 1; }
    }
    for (size_t i = 0; i < entrees.size(); i++)
        fprintf(sortie, "%llu\t%s\n", (unsigned long long)entrees[i].taille, entrees[i].nom.c_str());
    if (sortie != stdout) fclose(sortie);

    fprintf(stderr, "%lu entrees parcourues, %zu retenues.\n", total, entrees.size());
    CascCloseStorage(hStorage);
    return 0;
}

static int commande_extraire(int argc, char ** argv)
{
    if (argc < 5) { fprintf(stderr, "usage: extraire-casc extraire <racine> <motif> <dossier-sortie> [--plat]\n"); return 2; }

    std::string motif = motif_normalise(argv[3]);
    std::string dossier = argv[4];
    bool plat = false;
    for (int i = 5; i < argc; i++) if (strcmp(argv[i], "--plat") == 0) plat = true;

    HANDLE hStorage = ouvre(argv[2]);
    if (hStorage == NULL) return 1;
    cree_arborescence(dossier);

    std::vector<Entree> entrees = recense(hStorage, motif, NULL);
    std::vector<std::string> noms;
    for (size_t i = 0; i < entrees.size(); i++) noms.push_back(entrees[i].nom);
    fprintf(stderr, "%zu fichiers a extraire.\n", noms.size());

    size_t ok = extrait_tout(hStorage, noms, dossier, plat);
    fprintf(stderr, "%zu/%zu extraits vers %s\n", ok, noms.size(), dossier.c_str());
    CascCloseStorage(hStorage);
    return ok == noms.size() ? 0 : 1;
}

static int commande_extraire_liste(int argc, char ** argv)
{
    if (argc < 5) { fprintf(stderr, "usage: extraire-casc extraire-liste <racine> <noms.txt> <dossier-sortie> [--plat]\n"); return 2; }

    bool plat = false;
    for (int i = 5; i < argc; i++) if (strcmp(argv[i], "--plat") == 0) plat = true;

    FILE * entree = fopen(argv[3], "rb");
    if (entree == NULL) { fprintf(stderr, "lecture impossible : %s\n", argv[3]); return 1; }

    std::vector<std::string> noms;
    char ligne[MAX_PATH * 2];
    while (fgets(ligne, sizeof(ligne), entree) != NULL)
    {
        std::string s(ligne);
        while (!s.empty() && (s[s.size() - 1] == '\n' || s[s.size() - 1] == '\r')) s.erase(s.size() - 1);

        // Une ligne de `lister` porte la taille devant, separee par une tabulation.
        size_t tab = s.find('\t');
        if (tab != std::string::npos) s = s.substr(tab + 1);
        if (!s.empty()) noms.push_back(s);
    }
    fclose(entree);

    HANDLE hStorage = ouvre(argv[2]);
    if (hStorage == NULL) return 1;

    std::string dossier = argv[4];
    cree_arborescence(dossier);
    size_t ok = extrait_tout(hStorage, noms, dossier, plat);
    fprintf(stderr, "%zu/%zu extraits vers %s\n", ok, noms.size(), dossier.c_str());
    CascCloseStorage(hStorage);
    return ok == noms.size() ? 0 : 1;
}

int main(int argc, char ** argv)
{
    if (argc < 2)
    {
        fprintf(stderr,
            "extraire-casc -- lecture seule d'une installation Blizzard CASC\n"
            "\n"
            "  extraire-casc lister         <racine> [motif] [--sortie liste.txt]\n"
            "  extraire-casc extraire       <racine> <motif> <dossier-sortie> [--plat]\n"
            "  extraire-casc extraire-liste <racine> <noms.txt> <dossier-sortie> [--plat]\n"
            "\n"
            "<racine> est le dossier du jeu, celui qui porte .build.info.\n");
        return 2;
    }

    if (strcmp(argv[1], "lister") == 0)         return commande_lister(argc, argv);
    if (strcmp(argv[1], "extraire") == 0)       return commande_extraire(argc, argv);
    if (strcmp(argv[1], "extraire-liste") == 0) return commande_extraire_liste(argc, argv);

    fprintf(stderr, "commande inconnue : %s\n", argv[1]);
    return 2;
}
