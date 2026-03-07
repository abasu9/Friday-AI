document.addEventListener('DOMContentLoaded', () => {
    // Initialize Lenis for Smooth Scrolling
    const lenis = new Lenis({
        duration: 1.2,
        easing: (t) => Math.min(1, 1.001 - Math.pow(2, -10 * t)), 
        direction: 'vertical',
        gestureDirection: 'vertical',
        smooth: true,
        mouseMultiplier: 1,
        smoothTouch: false,
        touchMultiplier: 2,
        infinite: false,
    });

    function raf(time) {
        lenis.raf(time);
        requestAnimationFrame(raf);
    }
    requestAnimationFrame(raf);

    // Provide lenis instance globally for anchor links
    document.querySelectorAll('a[href^="#"]').forEach(anchor => {
        anchor.addEventListener('click', function (e) {
            e.preventDefault();
            lenis.scrollTo(this.getAttribute('href'));
        });
    });

    // Scroll Reveal Animations using Intersection Observer
    const revealElements = document.querySelectorAll('.scroll-reveal');

    const revealObserver = new IntersectionObserver((entries, observer) => {
        entries.forEach(entry => {
            if (entry.isIntersecting) {
                entry.target.classList.add('visible');
                // Optional: Stop observing once revealed if you only want it to animate once
                // observer.unobserve(entry.target);
            } else {
                // Remove the class when scrolled out of view to make it replayable
                entry.target.classList.remove('visible');
            }
        });
    }, {
        root: null,
        threshold: 0.15, // Trigger when 15% of the element is visible
        rootMargin: "0px 0px -50px 0px"
    });

    revealElements.forEach(el => revealObserver.observe(el));

    // Video Modal Logic
    const videoModal = document.getElementById('videoModal');
    const closeVideoModal = document.getElementById('closeVideoModal');
    const youtubeIframe = document.getElementById('youtubeIframe');
    const openVideoBtns = document.querySelectorAll('.open-video-modal');

    // The YouTube embed URL the user wants
    const videoSrc = "https://www.youtube.com/embed/XqZsoesa55w?autoplay=1";

    openVideoBtns.forEach(btn => {
        btn.addEventListener('click', (e) => {
            e.preventDefault();
            youtubeIframe.src = videoSrc;
            videoModal.classList.add('active');
            // Disable lenis scroll when modal is open
            lenis.stop();
        });
    });

    function closeModal() {
        videoModal.classList.remove('active');
        // Stop video playback by clearing src
        setTimeout(() => {
            youtubeIframe.src = "";
        }, 300); // Wait for transition to finish
        // Resume lenis scroll
        lenis.start();
    }

    closeVideoModal.addEventListener('click', closeModal);

    // Close on background click
    videoModal.addEventListener('click', (e) => {
        if (e.target === videoModal) {
            closeModal();
        }
    });

    // Close on Escape key
    document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape' && videoModal.classList.contains('active')) {
            closeModal();
        }
    });
});
