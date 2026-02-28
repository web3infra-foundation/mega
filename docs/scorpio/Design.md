# Preliminary proposal

> **⚠️ IMPORTANT: Scorpio has been moved to a separate repository**
>
> This documentation is kept for historical reference. For the latest Scorpio/ScorpioFS development, please visit:
> **https://github.com/web3infra-foundation/scorpiofs**

The following figure shows the preliminary design proposal of Scorpio. The `Repo Manager` is responsible for recording the mount point. 

Each Part Checkout corresponds to a `Checkout-Mounter`. It includes a Readonly Store and a `Mutable Overlay` and a `Readonly Store`.

![Struct](../images/scorpio.svg)

- `Mutable Overlay` as a mutable file layer, only keep the change of the specific checkout workspace. Like the [OverLay FS](https://en.wikipedia.org/wiki/OverlayFS),  it will be stacked on top of the read-only layer `Readonly Store`.

- `Readonly Store` saves a portion of the submitted git files or objects in the git repository and provides them to the upper mount point through multi-level caching.


Note that Scorpio may only requires using Tree and Blob objects, becase the  commit and tag objects are essentially not related to the file itself, but rather to version management.

