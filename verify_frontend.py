from playwright.sync_api import sync_playwright

def verify(page):
    print("Navigating to dashboard...")
    page.goto("http://localhost:5173")
    page.wait_for_load_state("networkidle")
    print("Taking dashboard screenshot...")
    page.screenshot(path="/home/jules/verification/dashboard.png")

    print("Navigating to settings...")
    page.click("text=Settings")
    page.wait_for_load_state("networkidle")
    print("Taking settings screenshot...")
    page.screenshot(path="/home/jules/verification/settings.png")

with sync_playwright() as p:
    browser = p.chromium.launch()
    page = browser.new_page()
    try:
        verify(page)
    except Exception as e:
        print(f"Error: {e}")
    finally:
        browser.close()
