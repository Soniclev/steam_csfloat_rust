# Csfloat.com to Steam price monitoring
This is a port to Rust of my internal project originally written in Python. The goal of this project is to examine the performance difference from the internal project, which uses Postgres+Redis and several Python services wrapped into Docker containers. In terms of this project, I aimed to minimize computational efforts by migrating the entire logic and storage inside the Rust program. Additionally, transitioning to Rust allows the program to make decisions about new items in a matter of milliseconds' time.

# New fee calculation algorithm
This project contains Rust module that [computes fees](src/fee.rs) efficiently in just four loop iterations, a method developed independently and recognized as optimal. It offers functions to add fees to a transaction and subtract fees from a total amount, ensuring minimal computational overhead while maintaining accuracy. Feel free to adopt in any programming languages.

# Note
Running this program may be challenging due to its integration with old internal project written in Python. Please be aware that I do not provide any warranty or support for setting up or running this project. However, feel free to explore the codebase for educational purposes.
