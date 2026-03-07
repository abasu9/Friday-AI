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

    // YouTube Modal Logic
    const videoModal = document.getElementById('videoModal');
    const modalOverlay = document.getElementById('modalOverlay');
    const youtubeIframe = document.getElementById('youtubeIframe');
    
    // Triggers
    const triggerBox = document.getElementById('hero-video-trigger');
    const navWatchBtn = document.getElementById('nav-watch-btn');
    
    const youtubeUrl = "https://www.youtube.com/embed/XqZsoesa55w?autoplay=1";

    function openModal() {
        if(videoModal && youtubeIframe) {
            videoModal.classList.add('active');
            youtubeIframe.src = youtubeUrl;
        }
    }

    function closeModal() {
        if(videoModal && youtubeIframe) {
            videoModal.classList.remove('active');
            youtubeIframe.src = ""; // Clear src to stop video
        }
    }

    if(triggerBox) triggerBox.addEventListener('click', openModal);
    if(navWatchBtn) navWatchBtn.addEventListener('click', openModal);
    if(modalOverlay) modalOverlay.addEventListener('click', closeModal);
});
