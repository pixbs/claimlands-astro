#import <Foundation/Foundation.h>

// Winit owns UIApplicationMain and all lifecycle callbacks. The Rust entrypoint
// must run on this main thread; a second UIApplicationMain would be invalid.
extern void claimlands_start(void);

int main(int argc, char *argv[]) {
    (void)argc;
    (void)argv;
    @autoreleasepool {
        claimlands_start();
    }
    // The iOS event loop does not return during a successful application launch.
    return 1;
}
