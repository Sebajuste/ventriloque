// Une API C minimale par-dessus CascLib, pour que Rust n'ait pas a connaitre son C++.
//
// CascLib expose deja du C, mais son enumeration marche par poignee et par structure de 300
// octets ; la traverser depuis Rust demanderait de recopier `CASC_FIND_DATA` a l'identique et de
// suivre ses evolutions. On la traverse ici, une fois, et Rust ne voit plus qu'un tableau
// d'index : nom, taille, lecture par nom.
//
// LE RECENSEMENT EST FAIT D'UN BLOC, avant toute lecture. CascLib ne garantit pas qu'une
// enumeration et une ouverture de fichier cohabitent sur le meme stockage, et un parcours coute
// une minute sur StarCraft II : autant le payer une fois et garder l'index.

#include <windows.h>
#include <string>
#include <vector>
#include "CascLib.h"

namespace {

struct Entree
{
    std::string nom;
    unsigned long long taille;
};

struct Stockage
{
    HANDLE poignee = NULL;
    std::string produit;
    std::vector<Entree> entrees;
};

} // namespace

extern "C" {

// La racine arrive en UTF-16 : CascLib est batie en Unicode pour que les chemins accentues --
// « D:\Jeux\Assassin's Creed » et pires -- ouvrent comme les autres.
void * casc_ouvrir(const wchar_t * racine)
{
    HANDLE poignee = NULL;
    if (!CascOpenStorage(racine, 0, &poignee))
        return NULL;

    Stockage * s = new Stockage();
    s->poignee = poignee;

    CASC_STORAGE_PRODUCT produit;
    memset(&produit, 0, sizeof(produit));
    if (CascGetStorageInfo(poignee, CascStorageProduct, &produit, sizeof(produit), NULL))
        s->produit = produit.szCodeName;

    return s;
}

void casc_fermer(void * poignee)
{
    Stockage * s = (Stockage *)poignee;
    if (s == NULL)
        return;
    if (s->poignee != NULL)
        CascCloseStorage(s->poignee);
    delete s;
}

const char * casc_produit(void * poignee)
{
    Stockage * s = (Stockage *)poignee;
    return (s != NULL) ? s->produit.c_str() : "";
}

// Rend le nombre d'entrees nommees du stockage. Idempotent : le second appel rend l'index deja
// etabli sans reparcourir.
size_t casc_recenser(void * poignee)
{
    Stockage * s = (Stockage *)poignee;
    if (s == NULL)
        return 0;
    if (!s->entrees.empty())
        return s->entrees.size();

    CASC_FIND_DATA trouve;
    HANDLE recherche = CascFindFirstFile(s->poignee, "*", &trouve, NULL);
    if (recherche == NULL)
        return 0;

    do
    {
        Entree e;
        e.nom = trouve.szFileName;
        e.taille = trouve.FileSize;
        s->entrees.push_back(e);
    }
    while (CascFindNextFile(recherche, &trouve));

    CascFindClose(recherche);
    return s->entrees.size();
}

const char * casc_nom(void * poignee, size_t rang)
{
    Stockage * s = (Stockage *)poignee;
    if (s == NULL || rang >= s->entrees.size())
        return "";
    return s->entrees[rang].nom.c_str();
}

unsigned long long casc_taille(void * poignee, size_t rang)
{
    Stockage * s = (Stockage *)poignee;
    if (s == NULL || rang >= s->entrees.size())
        return 0;
    return s->entrees[rang].taille;
}

// Lit un fichier en entier. `*sortie` est alloue ici et se rend par `casc_liberer`.
// Rend 0 si tout va bien, le code d'erreur de CascLib sinon.
int casc_lire(void * poignee, const char * nom, unsigned char ** sortie, size_t * taille)
{
    Stockage * s = (Stockage *)poignee;
    *sortie = NULL;
    *taille = 0;
    if (s == NULL)
        return ERROR_INVALID_HANDLE;

    HANDLE fichier = NULL;
    if (!CascOpenFile(s->poignee, nom, 0, CASC_OPEN_BY_NAME, &fichier))
        return (int)GetCascError();

    std::vector<unsigned char> tampon;
    unsigned char bloc[0x10000];
    int code = 0;
    for (;;)
    {
        DWORD lu = 0;
        if (!CascReadFile(fichier, bloc, sizeof(bloc), &lu))
        {
            DWORD err = GetCascError();
            if (err != ERROR_SUCCESS && err != ERROR_HANDLE_EOF)
                code = (int)err;
            break;
        }
        if (lu == 0)
            break;
        tampon.insert(tampon.end(), bloc, bloc + lu);
    }
    CascCloseFile(fichier);

    if (code != 0)
        return code;

    unsigned char * rendu = (unsigned char *)malloc(tampon.empty() ? 1 : tampon.size());
    if (rendu == NULL)
        return ERROR_NOT_ENOUGH_MEMORY;
    if (!tampon.empty())
        memcpy(rendu, &tampon[0], tampon.size());

    *sortie = rendu;
    *taille = tampon.size();
    return 0;
}

void casc_liberer(unsigned char * bloc)
{
    free(bloc);
}

} // extern "C"
