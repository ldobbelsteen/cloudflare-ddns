FROM python:alpine
COPY requirements.txt .
RUN pip install -r requirements
COPY cloudflare.py .
CMD ["python", "-u", "cloudflare.py", "/cloudflare/config.yaml"]
